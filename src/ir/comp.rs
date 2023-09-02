use super::{
    AddCtx, Cmp, CmpOp, Command, Ctx, Event, EventIdx, Expr, ExprIdx, Fact,
    IndexStore, Info, InfoIdx, InstIdx, Instance, Interned, InvIdx, Invoke,
    MutCtx, Param, ParamIdx, Port, PortIdx, Prop, PropIdx, Time, TimeSub,
};
use crate::{ast, utils::Idx};
use fil_derive::Ctx;
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone)]
/// Externally facing interface name information for components.
pub struct InterfaceSrc {
    pub name: ast::Id,
    pub ports: HashMap<PortIdx, ast::Id>,
    pub params: HashMap<ParamIdx, ast::Id>,
    pub events: HashMap<EventIdx, ast::Id>,
    pub interface_ports: HashMap<EventIdx, ast::Id>,
}

impl InterfaceSrc {
    pub fn new(name: ast::Id) -> Self {
        Self {
            name,
            ports: HashMap::new(),
            params: HashMap::new(),
            interface_ports: HashMap::new(),
            events: HashMap::new(),
        }
    }
}

#[derive(Default, Ctx, Clone)]
/// A IR component. If `is_ext` is true then this is an external component.
pub struct Component {
    #[ctx(Expr)]
    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    exprs: Interned<Expr>,
    #[ctx(Time)]
    #[add_ctx(Time)]
    /// Interned times
    times: Interned<Time>,
    #[ctx(Prop)]
    /// Interned propositions
    props: Interned<Prop>,

    // Component defined values.
    #[ctx(Port)]
    #[add_ctx(Port)]
    #[mut_ctx(Port)]
    /// Ports and bundles defined by the component.
    ports: IndexStore<Port>,
    #[ctx(Param)]
    #[add_ctx(Param)]
    #[mut_ctx(Param)]
    /// Parameters defined the component
    params: IndexStore<Param>,
    #[ctx(Event)]
    #[add_ctx(Event)]
    #[mut_ctx(Event)]
    /// Events defined by the component
    events: IndexStore<Event>,

    // Control flow entities
    #[ctx(Instance)]
    #[add_ctx(Instance)]
    #[mut_ctx(Instance)]
    /// Instances defined by the component
    instances: IndexStore<Instance>,
    #[ctx(Invoke)]
    #[add_ctx(Invoke)]
    #[mut_ctx(Invoke)]
    /// Invocations defined by the component
    invocations: IndexStore<Invoke>,

    /// Commands in the component
    pub cmds: Vec<Command>,

    #[ctx(Info)]
    #[add_ctx(Info)]
    /// Information tracked by the component
    info: IndexStore<Info>,
    /// Is this an external component
    pub is_ext: bool,
    /// Externally facing interface information, used to preserve interface in compilation.
    /// Must be `Some` for toplevel components and externals.
    pub src_info: Option<InterfaceSrc>,
    /// unannotated ports associated with this component
    pub unannotated_ports: Box<Vec<(ast::Id, u64)>>,
}

impl Component {
    pub fn new(is_ext: bool) -> Self {
        let mut comp = Self {
            is_ext,
            ..Default::default()
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
        panic!("{comp}\n{}", msg.to_string())
    }
}

/// Complex queries
impl Component {
    /// Returns an iterator over instances and the invocations that invoke them.
    pub fn inst_invoke_map(
        &self,
    ) -> impl Iterator<Item = (InstIdx, Vec<InvIdx>)> {
        self.invocations
            .iter()
            .map(|(idx, inv)| (inv.inst, idx))
            .into_group_map()
            .into_iter()
            .map(|(inst, invs)| (inst, invs.into_iter().collect()))
    }
}

/// Accessor methods
impl Component {
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

    pub fn inputs(&self) -> impl Iterator<Item = (PortIdx, &Port)> {
        self.ports.iter().filter(|(_, p)| p.is_sig_in())
    }

    pub fn outputs(&self) -> impl Iterator<Item = (PortIdx, &Port)> {
        self.ports.iter().filter(|(_, p)| p.is_sig_out())
    }
}

/// Queries over the component as a semantic entity
impl Component {
    /// The parameters in the signature of the component in the order they appear in the source
    pub fn sig_params(&self) -> Vec<ParamIdx> {
        let sig_params = self
            .params()
            .iter()
            .filter(|(_, param)| param.is_sig_owned())
            .map(|(idx, _)| idx)
            .collect_vec();

        sig_params
    }

    /// Returns all the phantom events in this component.
    /// A phantom event is an event without an interface port
    pub fn phantom_events(&self) -> impl Iterator<Item = EventIdx> + '_ {
        self.events()
            .idx_iter()
            .filter(|idx| !self.get(*idx).has_interface)
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
}

// =========== Context accessors for each type ===========

impl Component {
    pub fn resolve_prop(&mut self, prop: Prop) -> PropIdx {
        match prop {
            Prop::Cmp(cmp) => {
                let CmpOp { op, lhs, rhs } = cmp;
                let lhs = lhs.as_concrete(self).unwrap();
                let rhs = rhs.as_concrete(self).unwrap();
                match op {
                    Cmp::Gt => {
                        if lhs > rhs {
                            self.add(Prop::True)
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    Cmp::Eq => {
                        if lhs == rhs {
                            self.add(Prop::True)
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    Cmp::Gte => {
                        if lhs >= rhs {
                            self.add(Prop::True)
                        } else {
                            self.add(Prop::False)
                        }
                    }
                }
            }
            Prop::TimeCmp(_)
            | Prop::TimeSubCmp(_)
            | Prop::Implies(_, _)
            | Prop::True
            | Prop::False => self.add(prop),
            Prop::And(l, r) => {
                let l = self.resolve_prop(self.get(l).clone());
                let r = self.resolve_prop(self.get(r).clone());
                match (l.as_concrete(self), r.as_concrete(self)) {
                    (Some(l), Some(r)) => {
                        if l && r {
                            self.add(Prop::True)
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    (Some(l), None) => {
                        if l {
                            r
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    (None, Some(r)) => {
                        if r {
                            l
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    (None, None) => self.add(prop),
                }
            }
            Prop::Or(l, r) => {
                let l = self.resolve_prop(self.get(l).clone());
                let r = self.resolve_prop(self.get(r).clone());
                match (l.as_concrete(self), r.as_concrete(self)) {
                    (Some(l), Some(r)) => {
                        if l || r {
                            self.add(Prop::True)
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    (Some(l), None) => {
                        if l {
                            self.add(Prop::True)
                        } else {
                            r
                        }
                    }
                    (None, Some(r)) => {
                        if r {
                            self.add(Prop::True)
                        } else {
                            l
                        }
                    }
                    (None, None) => self.add(prop),
                }
            }
            Prop::Not(p) => {
                let p = self.resolve_prop(self.get(p).clone());
                match p.as_concrete(self) {
                    Some(p) => {
                        if p {
                            self.add(Prop::True)
                        } else {
                            self.add(Prop::False)
                        }
                    }
                    None => self.add(prop),
                }
            }
        }
    }

    /// Evaluates a function, assuming that all parms have been substituted for
    /// concrete expressions in monomorphization
    pub fn func(&mut self, expr: Expr) -> ExprIdx {
        //let expr = self.get(eidx);
        match expr {
            Expr::Concrete(_) => self.add(expr),
            Expr::Bin {..} => self.bin(expr),
            Expr::Param(_) => {
                self.internal_error(
                    "When evaluating a function expression, there should not be any parameters in it".to_string()
                )
            }
            Expr::Fn {op, args} => {
                let args = args.iter().map(|arg| { let arg = self.get(*arg); self.func(arg.clone()) }).collect_vec();
                let arg = args.get(0).unwrap().as_concrete(self).unwrap();
                match op {
                    ast::UnFn::Pow2 => {
                        self.add(Expr::Concrete(2u64.pow(arg as u32)))
                    }
                    ast::UnFn::Log2 => {
                        self.add(Expr::Concrete((arg as f64).log2().ceil() as u64))
                    }
                }
            }
        }
    }

    /// Simplifies an expression, assuming that all params have been substituted for
    /// concrete expressions in monomorphization
    pub fn bin(&mut self, expr: Expr) -> ExprIdx {
        match expr {
            Expr::Concrete(_) => self.add(expr),
            Expr::Bin { op, lhs, rhs } => {
                let lhs = self.bin(self.get(lhs).clone());
                let lhs = lhs.as_concrete(self).unwrap();
                let rhs = self.bin(self.get(rhs).clone());
                let rhs = rhs.as_concrete(self).unwrap();
                match op {
                    ast::Op::Add => self.add(Expr::Concrete(lhs + rhs)),
                    ast::Op::Mul => self.add(Expr::Concrete(lhs * rhs)),
                    ast::Op::Sub => self.add(Expr::Concrete(lhs - rhs)),
                    ast::Op::Div => self.add(Expr::Concrete(lhs / rhs)),
                    ast::Op::Mod => self.add(Expr::Concrete(lhs % rhs)),
                }
            }
            Expr::Param(pidx) => {
                self.internal_error(format!(
                    "When evaluating a binop expression, there should not be any parameters in it; found {pidx}")
                )
            }
            Expr::Fn { .. } => self.func(expr),
        }
    }
}

impl AddCtx<Expr> for Component {
    fn add(&mut self, val: Expr) -> ExprIdx {
        match &val {
            Expr::Param(_) | Expr::Concrete(_) => self.exprs.intern(val),
            Expr::Bin { op, lhs, rhs } => {
                let l = lhs.as_concrete(self);
                let r = rhs.as_concrete(self);
                let e = match (op, l, r) {
                    (ast::Op::Add, Some(0), None) => return *rhs,
                    (ast::Op::Add, None, Some(0))
                    | (ast::Op::Sub, None, Some(0)) => return *lhs,
                    (ast::Op::Mul, Some(0), None)
                    | (ast::Op::Div, Some(0), None)
                    | (ast::Op::Mul, None, Some(0)) => Expr::Concrete(0),
                    (ast::Op::Add, Some(l), None)
                    | (ast::Op::Mul, Some(l), None) => Expr::Bin {
                        op: *op,
                        lhs: *rhs,
                        rhs: self.exprs.intern(Expr::Concrete(l)),
                    },
                    (op, Some(l), Some(r)) => Expr::Concrete(match op {
                        ast::Op::Add => l + r,
                        ast::Op::Sub => l - r,
                        ast::Op::Mul => l * r,
                        ast::Op::Div => l / r,
                        ast::Op::Mod => l % r,
                    }),
                    _ => val,
                };
                self.exprs.intern(e)
            }
            Expr::Fn { op, args } => self.exprs.intern(
                args.iter()
                    .map(|arg| arg.as_concrete(self))
                    .collect::<Option<Vec<_>>>()
                    .map(|args| match op {
                        ast::UnFn::Pow2 => 1u64 << args[0],
                        ast::UnFn::Log2 => args[0].trailing_zeros() as u64,
                    })
                    .map_or(val, Expr::Concrete),
            ),
        }
    }
}

impl AddCtx<Prop> for Component {
    fn add(&mut self, val: Prop) -> Idx<Prop> {
        match val {
            Prop::True | Prop::False => self.props.intern(val),
            Prop::Not(p) => self.props.intern(if p.is_false(self) {
                Prop::True
            } else if p.is_true(self) {
                Prop::False
            } else if let Prop::Not(p) = self.get(p) {
                return *p;
            } else {
                Prop::Not(p)
            }),
            Prop::And(l, r) => {
                if l == r {
                    l
                } else if l.is_true(self) {
                    r
                } else if l.is_false(self) {
                    self.props.intern(Prop::False)
                } else if r.is_true(self) {
                    l
                } else if r.is_false(self) {
                    self.props.intern(Prop::False)
                } else {
                    let (l, r) = if l < r { (l, r) } else { (r, l) };
                    self.props.intern(Prop::And(l, r))
                }
            }
            Prop::Or(l, r) => {
                if l == r {
                    l
                } else if l.is_false(self) {
                    r
                } else if l.is_true(self) {
                    self.props.intern(Prop::True)
                } else if r.is_false(self) {
                    l
                } else if r.is_true(self) {
                    self.props.intern(Prop::True)
                } else {
                    let (l, r) = if l < r { (l, r) } else { (r, l) };
                    self.props.intern(Prop::Or(l, r))
                }
            }
            Prop::Implies(l, r) => {
                // If the proposition is false, then the implication is trivially true
                if l.is_false(self) {
                    // Warning because its not clear if this is ever expected behavior
                    log::warn!("A false proposition was created");
                    self.props.intern(Prop::True)
                } else if r.is_true(self) {
                    self.props.intern(Prop::True)
                } else if l.is_true(self) {
                    r
                } else {
                    self.props.intern(Prop::Implies(l, r))
                }
            }
            Prop::Cmp(CmpOp { op, lhs, rhs }) => self.props.intern(
                match (lhs.as_concrete(self), rhs.as_concrete(self)) {
                    (Some(l), Some(r)) => {
                        if match op {
                            Cmp::Gt => l > r,
                            Cmp::Gte => l >= r,
                            Cmp::Eq => l == r,
                        } {
                            Prop::True
                        } else {
                            Prop::False
                        }
                    }
                    _ => Prop::Cmp(CmpOp { op, lhs, rhs }),
                },
            ),
            Prop::TimeCmp(CmpOp { op, lhs, rhs }) => {
                let l = self.get(lhs);
                let r = self.get(rhs);

                if l.event == r.event {
                    self.add(Prop::Cmp(CmpOp {
                        op,
                        lhs: l.offset,
                        rhs: r.offset,
                    }))
                } else {
                    self.props.intern(Prop::TimeCmp(CmpOp { op, lhs, rhs }))
                }
            }
            Prop::TimeSubCmp(CmpOp { op, lhs, rhs }) => match (lhs, rhs) {
                (TimeSub::Unit(l), TimeSub::Unit(r)) => {
                    self.add(Prop::Cmp(CmpOp { op, lhs: l, rhs: r }))
                }
                (lhs, rhs) => {
                    self.props.intern(Prop::TimeSubCmp(CmpOp { op, lhs, rhs }))
                }
            },
        }
    }
}
