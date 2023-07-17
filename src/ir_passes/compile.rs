use std::{
    collections::{HashMap, HashSet},
    convert::identity,
    iter,
    path::PathBuf,
    rc::Rc,
};

use crate::{
    ir::{self, Ctx, Traversal},
    utils,
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
    ) -> Option<calyx::PortDef<CW>>
    where
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let raw_port = comp.get(port);

        if let ir::PortOwner::Sig { dir, .. } = &raw_port.owner {
            let name = Compile::port_name(ctx, comp, port);
            let mut attributes = calyx::Attributes::default();
            attributes.insert(calyx::BoolAttr::Data, 1);
            Some(calyx::PortDef {
                name: name.into(),
                width: width_transform(comp, comp.get(port).width),
                direction: calyx::Direction::from(dir).reverse(),
                attributes,
            })
        } else {
            None
        }
    }

    /// Converts an [ir::ExprIdx] into a [calyx::Width].
    /// Expects the [ir::ExprIdx] to either be a singular constant or an abstract variable.
    fn width(comp: &ir::Component, expr: ir::ExprIdx) -> calyx::Width {
        match comp.get(expr) {
            ir::Expr::Param(p) => calyx::Width::Param {
                value: Compile::param_name(comp, *p).into(),
            },
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
        is_comp: bool,
    ) -> Vec<calyx::PortDef<CW>>
    where
        WFU: Fn(u64) -> CW,
        WT: Fn(&ir::Component, ir::ExprIdx) -> CW,
    {
        let mut ports: Vec<_> = comp
            .ports()
            .idx_iter()
            .filter_map(|port| {
                Compile::port_def(ctx, comp, port, &width_transform)
            })
            .chain(comp.unannotated_ports().into_iter().map(|(name, width)| {
                calyx::PortDef {
                    name: name.as_ref().into(),
                    width: wfu(width),
                    direction: calyx::Direction::Input,
                    attributes: calyx::Attributes::default(),
                }
            }))
            .chain(
                comp.events()
                    .idx_iter()
                    .filter_map(|idx| Compile::interface_port_name(comp, idx))
                    .into_iter()
                    .map(|name| calyx::PortDef {
                        name: name.into(),
                        width: wfu(1),
                        direction: calyx::Direction::Input,
                        attributes: vec![(
                            calyx::Attribute::Unknown("fil_event".into()),
                            1,
                        )]
                        .try_into()
                        .unwrap(),
                    }),
            )
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

        if is_comp {
            // add remaining interface ports if not found
            for (attr, (name, width, dir)) in interface_ports {
                ports.push(calyx::PortDef {
                    name: (*name).into(),
                    width: wfu(*width),
                    direction: dir.clone(),
                    attributes: vec![*attr].try_into().unwrap(),
                });
            }
        }

        ports
    }

    /// Gets the component name given an [ir::Context] and an [ir::CompIdx]
    fn comp_name(ctx: &ir::Context, idx: ir::CompIdx) -> String {
        match &ctx.comps.get(idx).src_info {
            Some(src_info) => src_info.name.to_string(),
            None => format!("comp{}", idx.get()),
        }
    }

    fn port_name(
        ctx: &ir::Context,
        comp: &ir::Component,
        idx: ir::PortIdx,
    ) -> String {
        let p = comp.get(idx);

        match &p.owner {
            ir::PortOwner::Sig { .. } => {
                if let Some(src) = &comp.src_info {
                    src.ports.get(&idx).unwrap().to_string()
                } else {
                    format!("p{}", idx.get())
                }
            }
            ir::PortOwner::Inv { base, .. } => {
                base.unwrap(ctx, |c, p| Compile::port_name(ctx, c, p))
            }
            ir::PortOwner::Local => format!("p{}", idx.get()),
        }
    }

    fn param_name(comp: &ir::Component, idx: ir::ParamIdx) -> String {
        if let Some(src) = &comp.src_info {
            src.params.get(&idx).unwrap().to_string()
        } else {
            format!("pr{}", idx.get())
        }
    }

    // Gets the name associated with an event's interface port if it exists.
    fn interface_port_name(
        comp: &ir::Component,
        idx: ir::EventIdx,
    ) -> Option<String> {
        if comp.get(idx).has_interface {
            Some(if let Some(src) = &comp.src_info {
                src.interface_ports.get(&idx).unwrap().to_string()
            } else {
                format!("ev{}", idx.get())
            })
        } else {
            None
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
                .map(|(idx, _)| Compile::param_name(comp, idx).into())
                .collect(),
            signature: Compile::ports(
                ctx,
                comp,
                Compile::width_u64,
                Compile::width,
                false,
            ),
            attributes: calyx::Attributes::default(),
            is_comb: false,
            body: None,
            latency: None,
        }
    }

    /// Gets the maximum states for each event with an interface port in the component
    fn max_states(comp: &ir::Component) -> HashMap<ir::EventIdx, u64> {
        let mut event_map = HashMap::new();

        comp.times().iter().for_each(|(_, time)| {
            let nv = Compile::u64(comp, time.offset);
            if nv > *event_map.get(&time.event).unwrap_or(&0) {
                event_map.insert(time.event, nv);
            }
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
        let ports = Compile::ports(ctx, comp, identity, Compile::u64, true);
        let mut component = calyx::Component::new(
            Compile::comp_name(ctx, idx),
            ports,
            false,
            false,
            None,
        );
        component.attributes.insert(calyx::BoolAttr::NoInterface, 1);
        // main component
        if Some(idx) == ctx.entrypoint {
            log::debug!("Defining main component {idx}");
            component.attributes.insert(calyx::BoolAttr::TopLevel, 1);
        }

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

        cons.into_iter().for_each(|con| ctx.compile_connect(con));
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
        let main =
            frontend::ast::ComponentDef::new("main", false, None, vec![]);
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
    cidx: ir::CompIdx,
    comp: &'a ir::Component,
    builder: calyx::Builder<'a>,
    lib: &'a calyx::LibrarySignatures,
    binding: &'a mut Binding,
    fsms: HashMap<ir::EventIdx, Fsm>,
    /// Mapping from instances to cells
    instances: HashMap<ir::InstIdx, RRC<calyx::Cell>>,
    /// Mapping from invocation name to instance
    invokes: HashMap<ir::InvIdx, RRC<calyx::Cell>>,
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
            cidx: idx,
            comp: ctx.comps.get(idx),
            binding,
            builder,
            lib,
            instances: HashMap::new(),
            invokes: HashMap::new(),
            fsms: HashMap::new(),
        }
    }

    /// Attempts to declare an fsm (if not already declared) in the [Binding] stored by this [Context]
    /// and creates an [Fsm] from this [calyx::Component] FSM and stores it in the [Context]
    pub fn insert_fsm(&mut self, event: ir::EventIdx, states: u64) {
        // Construct an fsm iff the event is connected to an interface port
        if self.comp.get(event).has_interface {
            self.declare_fsm(states);

            // Construct the FSM
            let fsm = Fsm::new(event, states, self);
            self.fsms.insert(event, fsm);
        }
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    /// Returns no guard if the related event has no interface port.
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

        // return no guard the interface port does not exist.
        if self.comp.get(ev).has_interface {
            let fsm = self.fsms.get(&ev).unwrap();
            (Compile::u64(self.comp, start.offset)
                ..Compile::u64(self.comp, end.offset))
                .map(|st| fsm.get_port(st).into())
                .reduce(calyx::Guard::or)
                .unwrap()
        } else {
            calyx::Guard::True
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

        log::debug!("Compiling connect: {}", con);

        let (dst, _) = self.compile_port(dst.port);
        // assert!(!g.is_true(), "Destination port cannot have a guard.");
        let (src, g) = self.compile_port(src.port);
        let assign = self.builder.build_assignment(dst, src, g);
        self.builder.component.continuous_assignments.push(assign);
    }

    pub fn compile_port(
        &mut self,
        idx: ir::PortIdx,
    ) -> (RRC<calyx::Port>, calyx::Guard<calyx::Nothing>) {
        let port = self.comp.get(idx);

        let name = Compile::port_name(self.ctx, self.comp, idx);

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
        (cell.get(name), guard)
    }

    fn add_invoke(&mut self, invidx: ir::InvIdx) {
        let inv = self.comp.get(invidx);

        let cell = &self
            .instances
            .get(&inv.inst)
            .unwrap_or_else(|| panic!("Unknown instance: {}", inv.inst));

        for eb in inv.events.iter() {
            // If there is no interface port, no binding necessary
            if let Some(dst) =
                eb.base.unwrap(self.ctx, Compile::interface_port_name)
            {
                let ir::EventBind { arg: time, .. } = eb;

                let dst = cell.borrow().get(dst);

                let time = self.comp.get(*time);
                let offset = Compile::u64(self.comp, time.offset);
                let src = self.fsms.get(&time.event).unwrap().get_port(offset);

                let c = self.builder.add_constant(1, 1);

                let assign = self.builder.build_assignment(
                    dst,
                    c.borrow().get("out"),
                    calyx::Guard::Port(src),
                );
                self.builder.component.continuous_assignments.push(assign);
            }
        }

        self.invokes.insert(invidx, Rc::clone(cell));
    }

    fn add_instance(&mut self, idx: ir::InstIdx) {
        let inst = self.comp.get(idx);
        let inst_name = self.idx_name(idx);
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

    fn idx_name<T>(&self, idx: utils::Idx<T>) -> String
    where
        ir::Component: Ctx<T>,
        utils::Idx<T>: utils::IdxPre,
    {
        format!(
            "{}_{}{}",
            Compile::comp_name(self.ctx, self.cidx),
            <utils::Idx::<T> as utils::IdxPre>::prefix(),
            idx.get()
        )
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
            false,
            None,
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
        let comp = ctx.binding.fsm_comps.get(&states).unwrap();
        let cell = ctx.builder.add_component(
            ctx.idx_name(event),
            comp.name.to_string(),
            Binding::cell_to_port_def(&comp.signature),
        );

        if let Some(name) = Compile::interface_port_name(ctx.comp, event) {
            // gets the trigger port from the signature
            let sig = ctx.builder.component.signature.borrow();
            let trigger = sig.get(name);

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
