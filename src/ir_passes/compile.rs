use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    rc::Rc, convert::identity, iter,
};

use crate::{
    ast,
    errors::FilamentResult,
    ir::{self, Ctx, Traversal},
};
use calyx::structure;
use calyx_frontend as frontend;
use calyx_ir::{self as calyx, RRC};
use calyx_utils::CalyxResult;
use itertools::Itertools;

type AttrPair = (calyx::Attribute, u64);
const INTERFACE_PORTS: [(AttrPair, (&str, u64, calyx::Direction)); 2] = [
    (
        (calyx::Attribute::Bool(calyx::BoolAttr::Clk), 1),
        ("clk", 1, calyx::Direction::Input),
    ),
    (
        (calyx::Attribute::Bool(calyx::BoolAttr::Reset), 1),
        ("reset", 1, calyx::Direction::Input),
    ),
];

/// Compiles Filament into Calyx
/// Generates FSMs for each event
#[derive(Default)]
pub struct Compile;

impl Compile {
    fn port_def<CW, WT>(
        comp: &ir::Component,
        port: ir::PortIdx,
        width_transform: WT,
    ) -> calyx::PortDef<CW>
    where
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let raw_port = comp.get(port);
        if let ir::Info::Port { name, .. } = comp.get(raw_port.info) {
            let mut attributes = calyx::Attributes::default();
            attributes.insert(calyx::BoolAttr::Data, 1);
            calyx::PortDef {
                name: name.as_ref().into(),
                width: width_transform(comp, comp.get(port).width),
                direction: calyx::Direction::from(&raw_port.owner),
                attributes,
            }
        } else {
            unreachable!("Incorrect info type for port");
        }
    }

    /// Converts an [ir::ExprIdx] into a [calyx::Width].
    /// Expects the [ir::ExprIdx] to either be a singular constant or an abstract variable.
    fn width(comp: &ir::Component, expr: ir::ExprIdx) -> calyx::Width {
        match comp.get(expr) {
            ir::Expr::Param(p) => {
                if let ir::Info::Param { name, .. } =
                    comp.get(comp.get(*p).info)
                {
                    calyx::Width::Param {
                        value: name.as_ref().into(),
                    }
                } else {
                    unreachable!("Incorrect info type for param")
                }
            }
            ir::Expr::Concrete(val) => calyx::Width::Const { value: *val },
            ir::Expr::Bin { .. } | ir::Expr::Fn { .. } => {
                panic!("Port width must be a parameter or constant.")
            }
        }
    }

    /// Compiles an [ir::ExprIdx] into a [u64].
    /// Expects the [ir::ExprIdx] to be a single constant value
    fn u64(comp: &ir::Component, expr: ir::ExprIdx) -> u64 {
        match comp.get(expr) {
            ir::Expr::Concrete(val) => *val,
            ir::Expr::Param(..) | ir::Expr::Bin { .. } | ir::Expr::Fn { .. } => {
                panic!("Port width must be a constant.")
            }
        }
    }

    fn ports<CW, WFU, WT>(
        comp: &ir::Component,
        wfu: WFU, // Function that returns a CW type from a u64
        width_transform: WT,
    ) -> Vec<calyx::PortDef<CW>>
    where
        WFU: Fn(u64) -> CW,
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let mut ports: Vec<calyx::PortDef<CW>> = comp
            .ports()
            .idx_iter()
            .map(|port| Compile::port_def(comp, port, &width_transform))
            .chain(comp.interface_ports().into_iter().map(|name| {
                calyx::PortDef {
                    name: name.as_ref().into(),
                    width: wfu(1),
                    direction: calyx::Direction::Input,
                    attributes: vec![(
                        calyx::Attribute::Unknown("fil_event".into()),
                        1,
                    )]
                    .try_into()
                    .unwrap(),
                }
            }))
            .chain(comp.unannotated_ports().into_iter().map(|(name, width)| {
                calyx::PortDef {
                    name: name.as_ref().into(),
                    width: wfu(width),
                    direction: calyx::Direction::Input,
                    attributes: calyx::Attributes::default(),
                }
            }))
            .collect();

        let mut interface_ports =
            INTERFACE_PORTS.iter().collect::<HashSet<_>>();

        // add interface port attributes if necessary
        for pd in &mut ports {
            if let Some(pair) = INTERFACE_PORTS
                .iter()
                .find(|(_, (n, _, _))| *n == pd.name.as_ref())
            {
                assert!(
                    pair.1 .2 == pd.direction,
                    "Expected {} to be an {:?} port, got {:?} port.",
                    pair.1 .0,
                    pair.1 .2,
                    pd.direction
                );
                // TODO: Assert width equality
                interface_ports.remove(pair);
                pd.attributes.insert(pair.0 .0, pair.0 .1);
            }
        }

        // add remaining interface ports if not found
        for (attr, (name, width, dir)) in interface_ports {
            ports.push(calyx::PortDef {
                name: (*name).into(),
                width: wfu(*width),
                direction: dir.clone(),
                attributes: vec![*attr].try_into().unwrap(),
            });
        }

        ports
    }

    /// Gets the component name given an [ir::Context] and an [ir::CompIdx]
    fn comp_name(ctx: &ir::Context, idx: ir::CompIdx) -> calyx::Id {
        let comp = ctx.comps.get(idx);
        match comp.src_ext {
            // component is non-external, generate name from CompIdx
            None => idx.get().to_string().into(),
            Some(id) => id.as_ref().into()
        }
    }

    fn primitive(ctx: &ir::Context, idx: ir::CompIdx) -> calyx::Primitive {
        let comp = ctx.comps.get(idx);
        calyx::Primitive {
            name: Compile::comp_name(ctx, idx),
            params: comp.params().iter()
                .map(|(_, p)| {
                    if let ir::Info::Param {name, ..} = comp.get(p.info) {
                        name.as_ref().into()
                    } else {
                        unreachable!("Incorrect info type for parameter");
                    }
                }).collect(),
            signature: Compile::ports(comp, Compile::width_u64, Compile::width),
            attributes: calyx::Attributes::default(),
            is_comb: false,
            body: None,
        }
    }

    fn component(
        ctx: &ir::Context,
        idx: ir::CompIdx,
        sigs: &mut Binding,
        lib: &calyx::LibrarySignatures,
    ) -> FilamentResult<calyx::Component> {
        let comp = ctx.comps.get(idx);
        let ports = Compile::ports(comp, identity, Compile::u64);
        let mut component = calyx::Component::new(Compile::comp_name(ctx, idx), ports, false);
        component.attributes.insert(calyx::BoolAttr::NoInterface, 1);

        let builder = calyx::Builder::new(&mut component, lib).not_generated();
        let mut ctx = Context::new(sigs, builder, lib);
        todo!()
    }

    fn init(
        ctx: &ir::Context,
        externs: Vec<(&String, Vec<ir::CompIdx>)>,
    ) -> CalyxResult<calyx::Context> {
        let mut ws = frontend::Workspace::from_compile_lib()?;
        // Add externals
        ws.externs.extend(externs.into_iter().map(|(file, comps)| {
            (
                Some(PathBuf::from(file)),
                comps.into_iter().map(|idx| Compile::primitive(ctx, idx)).collect(),
            )
        }));

        // define a fake main component
        let main = frontend::ast::ComponentDef::new("main", false, vec![]);
        ws.components.push(main);
        let mut ctx = calyx::from_ast::ast_to_ir(ws)?;
        ctx.components.retain(|c| c.name != "main");
        Ok(ctx)
    }

    pub fn compile(ctx: ir::Context) {
        let externals = ctx
            .externals
            .iter()
            .map(|(k, v)| {
                (k, v.clone())
            })
            .collect();
        let mut calyx_ctx = Compile::init(&ctx, externals).unwrap_or_else(|e| {
            panic!("Error initializing calyx context: {:?}", e);
        });

        let mut bindings = Binding::default();

        let po = Traversal::from(ctx);

        po.apply_pre_order(|ctx, comp| {
            let comp = Compile::component(ctx, comp, &mut bindings, &calyx_ctx.lib)
                .unwrap_or_else(|e| {
                    panic!("Error compiling component: {:?}", e);
                });
            bindings.insert_comp(
                ast::Id::from(comp.name.id.as_str()),
                Rc::clone(&comp.signature),
            );
            calyx_ctx.components.push(comp);
        });

        calyx_ctx
            .components
            .extend(bindings.fsm_comps.into_values());

        let mut out = &mut std::io::stdout();
        calyx::Printer::write_context(&calyx_ctx, false, &mut out).unwrap();
    }
}

impl From<&ir::Direction> for calyx::Direction {
    fn from(value: &ir::Direction) -> Self {
        match value {
            ir::Direction::In => calyx::Direction::Input,
            ir::Direction::Out => calyx::Direction::Output,
        }
    }
}

impl From<&ir::PortOwner> for calyx::Direction {
    fn from(value: &ir::PortOwner) -> Self {
        match value {
            ir::PortOwner::Local => calyx::Direction::Inout,
            ir::PortOwner::Sig { dir } | ir::PortOwner::Inv { dir, .. } => {
                calyx::Direction::from(dir)
            }
        }
    }
}

/// Helper functions for type conversion, etc.
impl Compile {
    /// Converts a [u64] into a [calyx::Width]
    fn width_u64(value: u64) -> calyx::Width {
        calyx::Width::Const { value }
    }
}

/// Bindings associated with the current compilation context
#[derive(Default)]
struct Binding {
    // Component signatures
    comps: HashMap<ast::Id, RRC<calyx::Cell>>,
    /// Mapping to the component representing FSM with particular number of states
    fsm_comps: HashMap<u64, calyx::Component>,
}

impl Binding {
    pub fn insert_comp(&mut self, name: ast::Id, sig: RRC<calyx::Cell>) {
        self.comps.insert(name, sig);
    }

    /// Tries to get an fsm with a certain number of states.
    pub fn get_fsm(&self, states: &u64) -> &calyx::Component{
        self.fsm_comps.get(states).unwrap()
    }
}

struct Context<'a> {
    builder: calyx::Builder<'a>,
    lib: &'a calyx::LibrarySignatures,
    binding: &'a mut Binding,
}


impl<'a> Context<'a> {
    fn new(
        binding: &'a mut Binding,
        builder: calyx::Builder<'a>,
        lib: &'a calyx::LibrarySignatures,
    ) -> Self {
        Context {
            binding,
            builder,
            lib
        }
    }

    /// Creates a [calyx::Component] representing an FSM with a certain number of states to bind to each event.
    fn create_fsm(&mut self, states: u64) {
        let ports: Vec<calyx::PortDef<u64>> = (0..states)
            .map(|n| {
                (calyx::Id::from(format!("_{n}")), 1, calyx::Direction::Output).into()
            })
            .chain(INTERFACE_PORTS.iter().map(|(attr, pd)| {
                calyx::PortDef {
                    name: pd.0.into(),
                    width: pd.1,
                    direction: pd.2.clone(),
                    attributes: vec![*attr].try_into().unwrap(),
                }
            }))
            .chain(iter::once(
                (calyx::Id::from("go"), 1, calyx::Direction::Input).into(),
            ))
            .collect();
        let mut comp = calyx::Component::new(
            calyx::Id::from(format!("fsm_{}", states)),
            ports,
            false,
        );
        comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);
        let mut builder = calyx::Builder::new(&mut comp, self.lib).not_generated();

        // Add n-1 registers
        let regs = (0..states - 1)
            .map(|_| builder.add_primitive("r", "std_reg", &[1]))
            .collect_vec();

        // Constant signal
        structure!(builder;
            let signal_on = constant(1, 1);
        );
        // This component's interface
        let this = builder.component.signature.borrow();

        // _0 = go;
        let assign = builder.build_assignment(
            this.get("_0"),
            this.get("go"),
            calyx::Guard::True,
        );
        builder.component.continuous_assignments.push(assign);

        // For each register, add the following assignments:
        // rn.write_en = 1'd1;
        // rn.in = r{n-1}.out;
        // _n = rn.out;
        for idx in 0..states - 1 {
            let cell = regs[idx as usize].borrow();
            let write_assign = if idx == 0 {
                builder.build_assignment(
                    cell.get("in"),
                    this.get("go"),
                    calyx::Guard::True,
                )
            } else {
                let prev_cell = regs[(idx - 1) as usize].borrow();
                builder.build_assignment(
                    cell.get("in"),
                    prev_cell.get("out"),
                    calyx::Guard::True,
                )
            };
            let enable = builder.build_assignment(
                cell.get("write_en"),
                signal_on.borrow().get("out"),
                calyx::Guard::True,
            );
            let out = builder.build_assignment(
                this.get(format!("_{}", idx + 1)),
                cell.get("out"),
                calyx::Guard::True,
            );
            builder.component.continuous_assignments.extend([
                write_assign,
                enable,
                out,
            ]);
        }
        drop(this);
        self.binding.fsm_comps.insert(states, comp);
    }
}