use std::collections::HashMap;

use super::{
    CmpOp, Command, CompIdx, Ctx, Event, EventIdx, Expr, ExprIdx, Fact,
    IndexStore, Info, InfoIdx, InstIdx, Instance, Interned, InvIdx, Invoke,
    MutCtx, Param, ParamIdx, Port, PortIdx, Prop, PropIdx, Time, TimeIdx,
    TimeSub,
};
use crate::{ast, utils::Idx};

#[derive(Default)]
pub struct Context {
    pub comps: IndexStore<Component>,
    // Contains external components grouped by file name.
    pub externals: HashMap<String, Vec<CompIdx>>,
    pub entrypoint: Option<CompIdx>,
}

impl Context {
    pub fn is_main(&self, idx: CompIdx) -> bool {
        Some(idx) == self.entrypoint
    }
}

/// Queries corresponding signature [Event]s and [Port]s for their invoke counterparts
impl Context {
    /// Gets the [PortIdx] of the signature port of the invoked component corresponding
    /// to the given [PortIdx] in the invocation.
    pub fn get_component_port(
        &self,
        comp: &Component,
        idx: PortIdx,
    ) -> PortIdx {
        let port = comp.get(idx);
        let super::PortOwner::Inv { inv, dir } = &port.owner else {
            unreachable!("port owner is not an invocation");
        };
        let inv = comp.get(*inv);
        // finds the index of the port in the invocation
        let idx = inv.ports.iter()
            .filter(|&&p|
                matches!(&comp.get(p).owner, super::PortOwner::Inv { dir: d, .. } if d == dir)
            )
            .position(|&p| p == idx).unwrap();

        // get the foreign component
        let foreign = self.comps.get(comp.get(inv.inst).comp);

        foreign.ports().iter().filter(|(_, port)| {
            matches!(&port.owner, super::PortOwner::Sig { dir: d } if d == &dir.reverse())
        }).nth(idx).unwrap().0
    }

    /// Gets the [EventIdx] of the event in the invoked component corresponding
    /// to the given [EventBind] in the invocation.
    pub fn get_component_event(
        &self,
        comp: &Component,
        inv: InvIdx,
        idx: usize,
    ) -> EventIdx {
        let inv = comp.get(inv);
        // get the foreign component
        let foreign = self.comps.get(comp.get(inv.inst).comp);

        foreign.events().idx_iter().nth(idx).unwrap()
    }
}

/// Externally facing interface name information for components.
pub struct InterfaceSrc {
    pub name: ast::Id,
    pub ports: HashMap<PortIdx, ast::Id>,
    pub params: HashMap<ParamIdx, ast::Id>,
    pub interface_ports: HashMap<EventIdx, ast::Id>,
}

impl InterfaceSrc {
    pub fn new(name: ast::Id) -> Self {
        Self {
            name,
            ports: HashMap::new(),
            params: HashMap::new(),
            interface_ports: HashMap::new(),
        }
    }
}

/// A IR component. If `is_ext` is true then this is an external component.
pub struct Component {
    /// Identifier for the component
    idx: CompIdx,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    exprs: Interned<Expr>,
    /// Interned times
    times: Interned<Time>,
    /// Interned propositions
    props: Interned<Prop>,

    // Component defined values.
    /// Ports and bundles defined by the component.
    ports: IndexStore<Port>,
    /// Parameters defined the component
    params: IndexStore<Param>,
    /// Events defined by the component
    events: IndexStore<Event>,

    // Control flow entities
    /// Instances defined by the component
    instances: IndexStore<Instance>,
    /// Invocations defined by the component
    invocations: IndexStore<Invoke>,

    /// Commands in the component
    pub cmds: Vec<Command>,

    /// Information tracked by the component
    info: IndexStore<Info>,
    /// Is this an external component
    pub is_ext: bool,
    /// Externally facing interface information, used to preserve interface in compilation.
    /// Must be `Some` for toplevel components and externals.
    pub src_info: Option<InterfaceSrc>,
}

impl Component {
    pub fn new(idx: CompIdx, is_ext: bool) -> Self {
        let mut comp = Self {
            idx,
            is_ext,
            ports: IndexStore::default(),
            params: IndexStore::default(),
            events: IndexStore::default(),
            instances: IndexStore::default(),
            invocations: IndexStore::default(),
            info: IndexStore::default(),
            exprs: Interned::default(),
            times: Interned::default(),
            props: Interned::default(),
            cmds: Vec::default(),
            src_info: None,
        };
        // Allocate numbers and props now so we get reasonable indices.
        comp.num(0);
        comp.num(1);
        comp.add(Prop::False);
        comp.add(Prop::True);
        comp
    }

    /// Add a number to the context and get handle to it.
    pub fn num(&mut self, n: u64) -> ExprIdx {
        self.exprs.intern(Expr::Concrete(n))
    }

    /// Generates the propositions representing `a \subseteq b`
    pub fn subset_eq(
        &mut self,
        a: (ExprIdx, ExprIdx),
        b: (ExprIdx, ExprIdx),
    ) -> [PropIdx; 2] {
        let (a_lo, a_hi) = a;
        let (b_lo, b_hi) = b;
        let lo = a_lo.lte(b_lo, self);
        let hi = a_hi.gte(b_hi, self);
        [lo, hi]
    }

    /// Generate a asserted fact.
    /// Panics if the asserted fact is false.
    pub fn assert(&mut self, prop: PropIdx, info: InfoIdx) -> Option<Command> {
        if prop.is_true(self) {
            None
        } else {
            Some(Fact::assert(prop, info).into())
        }
    }

    /// Generate an assumed fact.
    /// Panics if the assumed fact is false.
    pub fn assume(&mut self, prop: PropIdx, info: InfoIdx) -> Option<Command> {
        if prop.is_false(self) {
            panic!("Attempted to assume false");
        } else if prop.is_true(self) {
            None
        } else {
            Some(Fact::assume(prop, info).into())
        }
    }

    /// Panic with an error message and display the current state of the Component. Prefer this over `panic!` when possible.
    pub fn internal_error<S: ToString>(&self, msg: S) -> ! {
        let comp = super::Printer::comp_str(self);
        panic!("{}\n{comp}", msg.to_string())
    }
}

/// Accessor methods
impl Component {
    pub fn idx(&self) -> CompIdx {
        self.idx
    }

    pub fn events(&self) -> &IndexStore<Event> {
        &self.events
    }

    pub fn ports(&self) -> &IndexStore<Port> {
        &self.ports
    }

    pub fn params(&self) -> &IndexStore<Param> {
        &self.params
    }

    pub fn invocations(&self) -> &IndexStore<Invoke> {
        &self.invocations
    }

    pub fn instances(&self) -> &IndexStore<Instance> {
        &self.instances
    }

    pub fn exprs(&self) -> &Interned<Expr> {
        &self.exprs
    }

    pub fn times(&self) -> &Interned<Time> {
        &self.times
    }

    pub fn props(&self) -> &Interned<Prop> {
        &self.props
    }
}

/// Queries over interned entities
impl Component {
    fn expr_params_acc(&self, expr: ExprIdx, acc: &mut Vec<ParamIdx>) {
        match self.get(expr) {
            Expr::Param(p) => acc.push(*p),
            Expr::Concrete(_) => (),
            Expr::Bin { lhs, rhs, .. } => {
                self.expr_params_acc(*lhs, acc);
                self.expr_params_acc(*rhs, acc);
            }
            Expr::Fn { args, .. } => {
                for arg in args.iter() {
                    self.expr_params_acc(*arg, acc);
                }
            }
        }
    }

    fn cmp_params_acc<T, F>(
        &self,
        cmp: &CmpOp<T>,
        acc: &mut Vec<ParamIdx>,
        add: F,
    ) where
        F: Fn(&T, &mut Vec<ParamIdx>),
    {
        let CmpOp { lhs, rhs, .. } = cmp;
        add(lhs, acc);
        add(rhs, acc);
    }

    fn prop_params_acc(&self, prop: PropIdx, acc: &mut Vec<ParamIdx>) {
        match self.get(prop) {
            Prop::True | Prop::False => (),
            Prop::Cmp(c) => self
                .cmp_params_acc(c, acc, |e, acc| self.expr_params_acc(*e, acc)),
            Prop::TimeCmp(c) => self.cmp_params_acc(c, acc, |t, acc| {
                self.expr_params_acc(self.get(*t).offset, acc)
            }),
            Prop::TimeSubCmp(c) => {
                self.cmp_params_acc(c, acc, |t, acc| match t {
                    TimeSub::Unit(e) => self.expr_params_acc(*e, acc),
                    TimeSub::Sym { l, r } => {
                        self.expr_params_acc(self.get(*l).offset, acc);
                        self.expr_params_acc(self.get(*r).offset, acc);
                    }
                })
            }
            Prop::Not(p) => self.prop_params_acc(*p, acc),
            Prop::And(l, r) | Prop::Or(l, r) | Prop::Implies(l, r) => {
                self.prop_params_acc(*l, acc);
                self.prop_params_acc(*r, acc);
            }
        }
    }

    /// Parameters mentioned within an expression
    pub fn expr_params(&self, expr: ExprIdx) -> Vec<ParamIdx> {
        let mut acc = Vec::new();
        self.expr_params_acc(expr, &mut acc);
        acc
    }

    /// Parameters mentioned within an expression
    pub fn prop_params(&self, prop: PropIdx) -> Vec<ParamIdx> {
        let mut acc = Vec::new();
        self.prop_params_acc(prop, &mut acc);
        acc
    }

    /// Unannotated Ports in the component
    pub fn unannotated_ports(&self) -> Vec<(&ast::Id, u64)> {
        self.info
            .iter()
            .filter_map(|(_, info)| match info {
                Info::UnannotatedPort { name, width } => Some((name, *width)),
                _ => None,
            })
            .collect()
    }
}

// =========== Context accessors for each type ===========

impl Ctx<Port> for Component {
    fn add(&mut self, val: Port) -> PortIdx {
        self.ports.add(val)
    }

    fn get(&self, idx: PortIdx) -> &Port {
        self.ports.get(idx)
    }
}

impl Ctx<Param> for Component {
    fn add(&mut self, val: Param) -> ParamIdx {
        self.params.add(val)
    }

    fn get(&self, idx: ParamIdx) -> &Param {
        self.params.get(idx)
    }
}

impl Ctx<Event> for Component {
    fn add(&mut self, val: Event) -> EventIdx {
        self.events.add(val)
    }

    fn get(&self, idx: EventIdx) -> &Event {
        self.events.get(idx)
    }
}

impl Ctx<Instance> for Component {
    fn add(&mut self, val: Instance) -> InstIdx {
        self.instances.add(val)
    }

    fn get(&self, idx: InstIdx) -> &Instance {
        self.instances.get(idx)
    }
}

impl Ctx<Invoke> for Component {
    fn add(&mut self, val: Invoke) -> InvIdx {
        self.invocations.add(val)
    }

    fn get(&self, idx: InvIdx) -> &Invoke {
        self.invocations.get(idx)
    }
}

impl Ctx<Expr> for Component {
    fn add(&mut self, val: Expr) -> ExprIdx {
        self.exprs.intern(val)
    }

    fn get(&self, idx: ExprIdx) -> &Expr {
        self.exprs.get(idx)
    }
}

impl Ctx<Time> for Component {
    fn add(&mut self, val: Time) -> TimeIdx {
        self.times.intern(val)
    }

    fn get(&self, idx: TimeIdx) -> &Time {
        self.times.get(idx)
    }
}

impl Ctx<Prop> for Component {
    fn add(&mut self, val: Prop) -> Idx<Prop> {
        self.props.intern(val)
    }

    fn get(&self, idx: Idx<Prop>) -> &Prop {
        self.props.get(idx)
    }
}

impl Ctx<Info> for Component {
    fn add(&mut self, val: Info) -> InfoIdx {
        self.info.add(val)
    }

    fn get(&self, idx: InfoIdx) -> &Info {
        self.info.get(idx)
    }
}

impl MutCtx<Port> for Component {
    fn get_mut(&mut self, idx: PortIdx) -> &mut Port {
        self.ports.get_mut(idx)
    }
}

impl MutCtx<Event> for Component {
    fn get_mut(&mut self, idx: EventIdx) -> &mut Event {
        self.events.get_mut(idx)
    }
}

impl MutCtx<Param> for Component {
    fn get_mut(&mut self, idx: ParamIdx) -> &mut Param {
        self.params.get_mut(idx)
    }
}

impl MutCtx<Invoke> for Component {
    fn get_mut(&mut self, idx: InvIdx) -> &mut Invoke {
        self.invocations.get_mut(idx)
    }
}

impl MutCtx<Instance> for Component {
    fn get_mut(&mut self, idx: InstIdx) -> &mut Instance {
        self.instances.get_mut(idx)
    }
}
