use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    iter,
    path::PathBuf,
    rc::Rc,
};

use crate::{
    ast,
    ir::{self, Ctx, Traversal},
};
use calyx::structure;
use calyx_frontend as frontend;
use calyx_ir::{self as calyx, RRC};
use calyx_utils::CalyxResult;
use itertools::Itertools;

type AttrPair = (calyx::Attribute, u64);
/// A set of interface ports that are required for all components.
pub const INTERFACE_PORTS: [(AttrPair, (&str, u64, calyx::Direction)); 2] = [
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
        ctx: &ir::Context,
        comp: &ir::Component,
        port: ir::PortIdx,
        width_transform: WT,
    ) -> calyx::PortDef<CW>
    where
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let raw_port = comp.get(port);
        let name = Compile::port_name(ctx, comp, port);
        let mut attributes = calyx::Attributes::default();
        attributes.insert(calyx::BoolAttr::Data, 1);
        calyx::PortDef {
            name: name.into(),
            width: width_transform(comp, comp.get(port).width),
            direction: calyx::Direction::from(&raw_port.owner),
            attributes,
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
            ir::Expr::Param(..)
            | ir::Expr::Bin { .. }
            | ir::Expr::Fn { .. } => {
                panic!("Port width must be a constant.")
            }
        }
    }

    fn ports<CW, WFU, WT>(
        ctx: &ir::Context,
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
            .map(|port| Compile::port_def(ctx, comp, port, &width_transform))
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
    fn comp_name(ctx: &ir::Context, idx: ir::CompIdx) -> String {
        let comp = ctx.comps.get(idx);
        match comp.src_ext {
            // component is non-external, generate name from CompIdx
            None => idx.to_string(),
            Some(id) => id.to_string(),
        }
    }

    fn port_name(ctx: &ir::Context, comp: &ir::Component, idx: ir::PortIdx) -> String {
        let p = comp.get(idx);
        
        let is_primitive = match p.owner {
            ir::PortOwner::Inv { inv, .. } => {
                let inv = comp.get(inv);
                let inst = comp.get(inv.inst);
                ctx.comps.get(inst.comp)
            },
            ir::PortOwner::Sig { .. }
            | ir::PortOwner::Local => comp,
        }.src_ext.is_some();

        if is_primitive {
            if let ir::Info::Port { name, .. } = comp.get(p.info) {
                name.as_ref().into()
            } else {
                unreachable!("Incorrect info type for parameter");
            }
        } else {
            idx.to_string()
        }
    }

    fn primitive(ctx: &ir::Context, idx: ir::CompIdx) -> calyx::Primitive {
        let comp = ctx.comps.get(idx);
        calyx::Primitive {
            name: Compile::comp_name(ctx, idx).into(),
            params: comp
                .params()
                .iter()
                .filter(|(_, p)| ir::ParamOwner::Sig == p.owner)
                .map(|(_, p)| {
                    if let ir::Info::Param { name, .. } = comp.get(p.info) {
                        name.as_ref().into()
                    } else {
                        unreachable!("Incorrect info type for parameter");
                    }
                })
                .collect(),
            signature: Compile::ports(ctx, comp, Compile::width_u64, Compile::width),
            attributes: calyx::Attributes::default(),
            is_comb: false,
            body: None,
        }
    }

    fn max_state_from_liveness(
        comp: &ir::Component,
        live: &ir::Liveness,
        event_map: &mut HashMap<ir::EventIdx, u64>,
    ) {
        assert!(
            &ir::Expr::Concrete(1) == comp.get(live.len),
            "Port bundles should have been compiled away."
        );
        for time in [live.range.start, live.range.end] {
            let time = comp.get(time);
            let nv = Compile::u64(comp, time.offset);
            if nv > *event_map.get(&time.event).unwrap_or(&0) {
                event_map.insert(time.event, nv);
            }
        }
    }

    /// Gets the maximum states for each event with an interface port in the component
    fn max_states(comp: &ir::Component) -> HashMap<ir::EventIdx, u64> {
        let mut event_map = HashMap::new();

        comp.ports()
            .idx_iter()
            .filter_map(|port| match comp.get(port) {
                ir::Port {
                    owner:
                        ir::PortOwner::Sig {
                            dir: ir::Direction::In,
                        },
                    live,
                    ..
                }
                | ir::Port {
                    owner:
                        ir::PortOwner::Inv {
                            dir: ir::Direction::Out,
                            ..
                        },
                    live,
                    ..
                } => Some(live),
                _ => None,
            })
            .for_each(|port| {
                Compile::max_state_from_liveness(comp, port, &mut event_map);
            });

        event_map
    }

    fn component(
        ctx: &ir::Context,
        idx: ir::CompIdx,
        bind: &mut Binding,
        lib: &calyx::LibrarySignatures,
    ) -> calyx::Component {
        log::debug!("Compiling component {idx}");
        let comp = ctx.comps.get(idx);
        let ports = Compile::ports(ctx, comp, identity, Compile::u64);
        let mut component =
            calyx::Component::new(Compile::comp_name(ctx, idx), ports, false);
        component.attributes.insert(calyx::BoolAttr::NoInterface, 1);

        let builder = calyx::Builder::new(&mut component, lib).not_generated();
        let mut ctx = Context::new(ctx, idx, bind, builder, lib);

        // Construct all the FSMs
        for (event, states) in Compile::max_states(comp) {
            ctx.insert_fsm(event, states);
        }

        let mut cons = vec![];

        for cmd in &comp.cmds {
            match cmd {
                ir::Command::Instance(idx) => ctx.add_instance(*idx),
                ir::Command::Invoke(idx) => ctx.add_invoke(*idx),
                ir::Command::EventBind(eb) => ctx.compile_eventbind(eb),
                ir::Command::Connect(connect) => cons.push(connect), // connects will be compiled later
                ir::Command::ForLoop(_) => {
                    unreachable!("For loops should have been compiled away.")
                }
                ir::Command::If(_) => {
                    unreachable!("Ifs should have been compiled away.")
                }
                ir::Command::Fact(_) => (),
            }
        }

        cons.into_iter().for_each(
            |con| ctx.compile_connect(con)
        );
        component
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
                comps
                    .into_iter()
                    .map(|idx| Compile::primitive(ctx, idx))
                    .collect(),
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
        let externals =
            ctx.externals.iter().map(|(k, v)| (k, v.clone())).collect();
        let mut calyx_ctx =
            Compile::init(&ctx, externals).unwrap_or_else(|e| {
                panic!("Error initializing calyx context: {:?}", e);
            });

        let mut bindings = Binding::default();

        let po = Traversal::from(ctx);

        po.apply_pre_order(|ctx, idx| {
            let comp =
                Compile::component(ctx, idx, &mut bindings, &calyx_ctx.lib);
            bindings.insert(idx, Rc::clone(&comp.signature));
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
    comps: HashMap<ir::CompIdx, RRC<calyx::Cell>>,
    /// Mapping to the component representing FSM with particular number of states
    fsm_comps: HashMap<u64, calyx::Component>,
}

impl Binding {
    /// Inserts a [calyx::Cell] into the binding
    pub fn insert(&mut self, name: ir::CompIdx, sig: RRC<calyx::Cell>) {
        self.comps.insert(name, sig);
    }

    /// Gets a [calyx::Cell]'s signature from an [ir::CompIdx]
    pub fn get(&self, idx: &ir::CompIdx) -> Option<Vec<calyx::PortDef<u64>>> {
        self.comps.get(idx).map(Self::cell_to_port_def)
    }

    /// Converts a cell to a list of port definitions
    pub fn cell_to_port_def(cr: &RRC<calyx::Cell>) -> Vec<calyx::PortDef<u64>> {
        let cell = cr.borrow();
        cell.ports()
            .iter()
            .map(|pr| {
                let port = pr.borrow();
                // Reverse port direction because signature refers to internal interface.
                (port.name, port.width, port.direction.reverse()).into()
            })
            .collect()
    }
}

struct Context<'a> {
    ctx: &'a ir::Context,
    comp: &'a ir::Component,
    builder: calyx::Builder<'a>,
    lib: &'a calyx::LibrarySignatures,
    binding: &'a mut Binding,
    fsms: HashMap<ir::EventIdx, Fsm>,
    /// Mapping from instances to cells
    instances: HashMap<ir::InstIdx, RRC<calyx::Cell>>,
    /// Mapping from invocation name to instance
    invokes: HashMap<ir::InvIdx, RRC<calyx::Cell>>,
    /// Binds events to the [calyx::Port] that they are connected to, used to connect interface ports.
    eventbinds: HashMap<ir::EventIdx, RRC<calyx::Port>>,
}

impl<'a> Context<'a> {
    fn new(
        ctx: &'a ir::Context,
        idx: ir::CompIdx,
        binding: &'a mut Binding,
        builder: calyx::Builder<'a>,
        lib: &'a calyx::LibrarySignatures,
    ) -> Self {
        Context {
            ctx,
            comp: ctx.comps.get(idx),
            binding,
            builder,
            lib,
            instances: HashMap::new(),
            invokes: HashMap::new(),
            fsms: HashMap::new(),
            eventbinds: HashMap::new(),
        }
    }

    /// Attempts to declare an fsm (if not already declared) in the [Binding] stored by this [Context]
    /// and creates an [Fsm] from this [calyx::Component] FSM and stores it in the [Context]
    pub fn insert_fsm(&mut self, event: ir::EventIdx, states: u64) {
        self.declare_fsm(states);

        // Construct the FSM
        let fsm = Fsm::new(event, states, self);
        self.fsms.insert(event, fsm);
    }

    /// Gets the interface port connected to an event
    /// Panics if the event is not connected to an interface port
    fn get_interface_port(&self, event: ir::EventIdx) -> Option<RRC<calyx::Port>> {
        let info = self.comp.get(event).interface_port?;
        if let ir::Info::InterfacePort { name, .. } = self.comp.get(info) {
            Some(self.get_port_name(name))
        } else {
            unreachable!("Interface port had incorrect info type.")
        }
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    fn compile_range(
        &mut self,
        range: &ir::Range,
    ) -> calyx::Guard<calyx::Nothing> {
        let start = self.comp.get(range.start);
        let end = self.comp.get(range.end);

        assert!(
            start.event == end.event,
            "Range `{range}` cannot be represented as a simple offset"
        );

        let ev = start.event;

        let fsm = self.fsms.get(&ev).unwrap();
        (Compile::u64(self.comp, start.offset)
            ..Compile::u64(self.comp, end.offset))
            .map(|st| fsm.get_port(st).into())
            .reduce(calyx::Guard::or)
            .unwrap()
    }

    fn compile_eventbind(&mut self, eb: &ir::EventBind) {
        let ir::EventBind {
            event: dst,
            arg: time,
            ..
        } = eb;
        let time = self.comp.get(*time);
        // If there is no interface port, no binding necessary
        if let Some(dst) = self.get_interface_port(*dst) {
            let offset = Compile::u64(self.comp, time.offset);
            let src = self.fsms.get(&time.event).unwrap().get_port(offset);

            let assign =
                self.builder.build_assignment(dst, src, calyx::Guard::True);
            self.builder.component.continuous_assignments.push(assign);
        }
    }

    fn compile_connect(&mut self, con: &ir::Connect) {
        let ir::Connect { dst, src, .. } = con;

        assert!(
            Compile::u64(self.comp, src.start) == 0
                && Compile::u64(self.comp, src.end) == 1,
            "Port bundles should have been compiled away."
        );

        assert!(
            Compile::u64(self.comp, dst.start) == 0
                && Compile::u64(self.comp, dst.end) == 1,
            "Port bundles should have been compiled away."
        );

        let (dst, g) = self.compile_port(dst.port);
        assert!(!g.is_true(), "Destination port cannot have a guard.");
        let (src, g) = self.compile_port(src.port);
        let assign = self.builder.build_assignment(src, dst, g);
        self.builder.component.continuous_assignments.push(assign);
    }

    pub fn compile_port(
        &mut self,
        idx: ir::PortIdx,
    ) -> (RRC<calyx::Port>, calyx::Guard<calyx::Nothing>) {
        let port = self.comp.get(idx);

        let guard = self.compile_range(&port.live.range);
        let cell = match port.owner {
            ir::PortOwner::Sig { .. } => {
                self.builder.component.signature.borrow()
            }
            ir::PortOwner::Inv { inv, .. } => self.invokes[&inv].borrow(),
            ir::PortOwner::Local => {
                unreachable!("Local ports should have been eliminated.")
            }
        };
        (cell.get(Compile::port_name(self.ctx, self.comp, idx)), guard)
    }

    /// Compiles an [ast::Id] port name to a [calyx::Port] reference
    pub fn get_port_name(&self, id: &ast::Id) -> RRC<calyx::Port> {
        let cell = self.builder.component.signature.borrow();
        cell.get(id.as_ref())
    }

    fn add_invoke(&mut self, idx: ir::InvIdx) {
        let inv = self.comp.get(idx);
        let comp = self.ctx.comps.get(self.comp.get(inv.inst).comp);
        let inst = self.instances.get(&inv.inst).unwrap().borrow();

        // Generate the assignment for each interface port
        for (evt, name) in comp.events().iter().filter_map(|(i, e)| {
            e.interface_port.map(|p| {
                if let ir::Info::InterfacePort { name, .. } = comp.get(p) {
                    (i, name)
                } else {
                    unreachable!("Interface port had incorrect info type.");
                }
            })
        }) {
            let dst = inst.get(name.as_ref());
            let src = self.eventbinds.get(&evt).unwrap().clone();

            let assign =
                self.builder.build_assignment(dst, src, calyx::Guard::True);
            self.builder.component.continuous_assignments.push(assign);
        }

        let cell = &self
            .instances
            .get(&inv.inst)
            .unwrap_or_else(|| panic!("Unknown instance: {}", inv.inst));
        self.invokes.insert(idx, Rc::clone(cell));
    }

    fn add_instance(&mut self, idx: ir::InstIdx) {
        let inst = self.comp.get(idx);
        let inst_name = inst.to_string();
        let comp_name = Compile::comp_name(self.ctx, inst.comp);
        let cell = if let Some(sig) = self.get_sig(&inst.comp) {
            self.builder.add_component(
                inst_name, // non-primitive component
                comp_name, sig,
            )
        } else {
            let conc_bind = inst
                .params
                .iter()
                .map(|v| Compile::u64(self.comp, *v))
                .collect_vec();
            self.builder.add_primitive(inst_name, comp_name, &conc_bind)
        };

        cell.borrow_mut()
            .attributes
            .insert(calyx::BoolAttr::Data, 1);
        self.instances.insert(idx, cell);
    }

    fn get_sig(&self, comp: &ir::CompIdx) -> Option<Vec<calyx::PortDef<u64>>> {
        self.binding.get(comp)
    }

    /// Creates a [calyx::Component] representing an FSM with a certain number of states to bind to each event.
    /// Adds component to the [Binding] held by this component.
    fn declare_fsm(&mut self, states: u64) {
        if self.binding.fsm_comps.contains_key(&states) {
            return;
        }

        let ports: Vec<calyx::PortDef<u64>> = (0..states)
            .map(|n| {
                (
                    calyx::Id::from(format!("_{n}")),
                    1,
                    calyx::Direction::Output,
                )
                    .into()
            })
            .chain(INTERFACE_PORTS.iter().map(|(attr, pd)| calyx::PortDef {
                name: pd.0.into(),
                width: pd.1,
                direction: pd.2.clone(),
                attributes: vec![*attr].try_into().unwrap(),
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
        let mut builder =
            calyx::Builder::new(&mut comp, self.lib).not_generated();

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

/// A wrapper for a [calyx::Component] representing a finite state machine.
struct Fsm {
    /// The [calyx::Component] representing this fsm.
    cell: RRC<calyx::Cell>,
}

impl Fsm {
    fn new(event: ir::EventIdx, states: u64, ctx: &mut Context) -> Self {
        let event = ctx.comp.get(event);
        let comp = ctx.binding.fsm_comps.get(&states).unwrap();
        let cell = ctx.builder.add_component(
            format!("{event}"),
            comp.name.to_string(),
            Binding::cell_to_port_def(&comp.signature),
        );

        if let ir::Info::InterfacePort { name, .. } =
            ctx.comp.get(event.interface_port.unwrap())
        {
            // gets the trigger port from the signature
            let sig = ctx.builder.component.signature.borrow();
            let trigger = sig.get(name.as_ref());

            // Connect the trigger port to the instance
            let go_assign = ctx.builder.build_assignment(
                cell.borrow().get("go"),
                trigger,
                calyx::Guard::True,
            );
            ctx.builder.component.continuous_assignments.push(go_assign);
            Fsm { cell }
        } else {
            unreachable!("Info should be an interface port");
        }
    }

    fn get_port(&self, state: u64) -> RRC<calyx::Port> {
        self.cell.borrow().get(format!("_{}", state))
    }
}
