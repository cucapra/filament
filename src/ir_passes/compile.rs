use std::{collections::HashMap, path::PathBuf, rc::Rc};

use crate::{
    ast,
    errors::FilamentResult,
    ir::{self, Ctx, Traversal},
};
use calyx_frontend as frontend;
use calyx_ir::{self as calyx, RRC};
use calyx_utils::CalyxResult;

const INTERFACE_PORTS: [(calyx::BoolAttr, (&str, u64, calyx::Direction)); 2] = [
    (calyx::BoolAttr::Clk, ("clk", 1, calyx::Direction::Input)),
    (
        calyx::BoolAttr::Reset,
        ("reset", 1, calyx::Direction::Input),
    ),
];

/// Compiles Filament into Calyx
/// Generates FSMs for each
#[derive(Default)]
pub struct Compile;

impl Compile {
    fn port_def<CW, WT>(
        comp: &ir::Component,
        port: ir::PortIdx,
        width_transform: WT,
    ) -> calyx::PortDef<CW>
    where
        WT: Fn(&ir::Component, ir::PortIdx) -> CW,
    {
        let raw_port = comp.get(port);
        if let ir::Info::Port { name, .. } = comp.get(raw_port.info) {
            let mut attributes = calyx::Attributes::default();
            attributes.insert(calyx::BoolAttr::Data, 1);
            calyx::PortDef {
                name: name.as_ref().into(),
                width: width_transform(comp, port),
                direction: calyx::Direction::from(&raw_port.owner),
                attributes,
            }
        } else {
            unreachable!("Incorrect info type for port");
        }
    }

    fn width(comp: &ir::Component, port: ir::PortIdx) -> calyx::Width {
        match comp.get(comp.get(port).width) {
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

    fn primitive(comp: &ir::Component) -> calyx::Primitive {
        calyx::Primitive {
            name: match comp.src_ext {
                None => unreachable!(
                    "Attempting to generate primitive from non-external component."
                ),
                Some(id) => id.as_ref().into()
            },
            params: comp.params().iter()
                .map(|(_, p)| {
                    if let ir::Info::Param {name, ..} = comp.get(p.info) {
                        name.as_ref().into()
                    } else {
                        unreachable!("Incorrect info type for parameter");
                    }
                }).collect(),
            signature: comp.ports().idx_iter()
                .map(|port| {
                    Compile::port_def(comp, port, Compile::width)
                }).collect(),
            attributes: todo!(),
            is_comb: todo!(),
            body: todo!(),
        }
    }

    fn component(
        comp: &ir::Component,
        sigs: &mut Binding,
        lib: &calyx::LibrarySignatures,
    ) -> FilamentResult<calyx::Component> {
        todo!()
    }

    fn init(
        externs: Vec<(&String, Vec<&ir::Component>)>,
    ) -> CalyxResult<calyx::Context> {
        let mut ws = frontend::Workspace::from_compile_lib()?;
        // Add externals
        ws.externs.extend(externs.into_iter().map(|(file, comps)| {
            (
                Some(PathBuf::from(file)),
                comps.into_iter().map(Compile::primitive).collect(),
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
                (k, v.iter().map(|idx| ctx.comps.get(*idx)).collect())
            })
            .collect();
        let mut calyx_ctx = Compile::init(externals).unwrap_or_else(|e| {
            panic!("Error initializing calyx context: {:?}", e);
        });

        let mut bindings = Binding::default();

        let po = Traversal::from(ctx);

        po.apply_pre_order(|ctx, comp| {
            let comp = ctx.comps.get(comp);
            let comp = Compile::component(comp, &mut bindings, &calyx_ctx.lib)
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

impl From<ir::Expr> for calyx::Width {
    fn from(value: ir::Expr) -> Self {
        match value {
            ir::Expr::Param(p) => todo!(),
            ir::Expr::Concrete(_) => todo!(),
            ir::Expr::Bin { op, lhs, rhs } => todo!(),
            ir::Expr::Fn { op, args } => todo!(),
        }
    }
}

/// Bindings associated with the current compilation context
#[derive(Default)]
struct Binding {
    // Component signatures
    comps: HashMap<ast::Id, RRC<calyx::Cell>>,
    /// Mapping to the component representing FSM with particular number of states
    pub fsm_comps: HashMap<u64, calyx::Component>,
}

impl Binding {
    pub fn insert_comp(&mut self, name: ast::Id, sig: RRC<calyx::Cell>) {
        self.comps.insert(name, sig);
    }
}

struct Context<'a> {
    builder: calyx::Builder<'a>,
    lib: &'a calyx::LibrarySignatures,
    bindings: Binding,
}
