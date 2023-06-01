use super::{
    CmpOp, Command, CompIdx, Ctx, Event, EventIdx, Expr, ExprIdx, Fact,
    IndexStore, InstIdx, Instance, Interned, InvIdx, Invoke, Param, ParamIdx,
    Port, PortIdx, Prop, PropIdx, Time, TimeIdx, TimeSub,
};
use crate::{ast, utils::Idx};
use itertools::Itertools;

#[derive(Default)]
pub struct Context {
    pub comps: IndexStore<CompOrExt>,
}

impl Ctx<CompOrExt> for Context {
    fn add(&mut self, val: CompOrExt) -> Idx<CompOrExt> {
        self.comps.add(val)
    }

    fn get(&self, idx: Idx<CompOrExt>) -> &CompOrExt {
        self.comps.get(idx)
    }
}

pub enum CompOrExt {
    Comp(Component),
    Ext(External),
}

/// An external component.
pub struct External {
    pub idx: CompIdx,
    pub sig: ast::Signature,
}

pub struct Component {
    pub idx: CompIdx,
    // Component defined values.
    /// Ports and bundles defined by the component.
    pub ports: IndexStore<Port>,
    /// Parameters defined the component
    pub params: IndexStore<Param>,
    /// Events defined by the component
    pub events: IndexStore<Event>,

    /// Instances defined by the component
    pub instances: IndexStore<Instance>,
    /// Invocations defined by the component
    pub invocations: IndexStore<Invoke>,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    pub exprs: Interned<Expr>,
    /// Interned times
    pub times: Interned<Time>,
    /// Interned propositions
    pub props: Interned<Prop>,

    /// Commands in the component
    pub cmds: Vec<Command>,
}

impl Component {
    pub fn new(idx: CompIdx) -> Self {
        let mut comp = Self {
            idx,
            ports: IndexStore::default(),
            params: IndexStore::default(),
            events: IndexStore::default(),
            instances: IndexStore::default(),
            invocations: IndexStore::default(),
            exprs: Interned::default(),
            times: Interned::default(),
            props: Interned::default(),
            cmds: Vec::default(),
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

    /// Generate a asserted fact.
    /// Panics if the asserted fact is false.
    pub fn assert(&self, prop: PropIdx) -> Fact {
        if prop.is_false(self) {
            panic!("Attempted to assert false");
        }
        Fact::assert(prop)
    }

    /// Generate an assumed fact.
    /// Panics if the assumed fact is false.
    pub fn assume(&self, prop: PropIdx) -> Fact {
        if prop.is_false(self) {
            panic!("Attempted to assume false");
        }
        Fact::assume(prop)
    }
}

/// Implement methods to display various values bound by the component
impl Component {
    /// Display an expression by recursively displaying its subexpressions.
    pub fn display_expr(&self, expr: ExprIdx) -> String {
        match self.get(expr) {
            Expr::Param(p) => format!("{p}"),
            Expr::Concrete(n) => format!("{n}"),
            Expr::Bin { op, lhs, rhs } => {
                let op_str = match op {
                    ast::Op::Add => "+",
                    ast::Op::Sub => "-",
                    ast::Op::Mul => "*",
                    ast::Op::Div => "/",
                    ast::Op::Mod => "%",
                };
                format!(
                    "({l} {op} {r})",
                    l = self.display_expr(*lhs),
                    op = op_str,
                    r = self.display_expr(*rhs)
                )
            }
            Expr::Fn { op, args } => {
                let fn_str = match op {
                    ast::UnFn::Pow2 => "pow2",
                    ast::UnFn::Log2 => "log2",
                };
                format!(
                    "({fn_str} {args})",
                    args =
                        args.iter().map(|a| self.display_expr(*a)).join(", ")
                )
            }
        }
    }

    fn display_cmp<T>(
        &self,
        cmp: &CmpOp<T>,
        print_base: impl Fn(T) -> String,
    ) -> String
    where
        T: Clone,
    {
        let CmpOp { op, lhs, rhs } = cmp;
        let op_str = match op {
            super::Cmp::Gt => ">",
            super::Cmp::Gte => ">=",
            super::Cmp::Eq => "=",
        };
        format!(
            "({l} {op} {r})",
            l = print_base(lhs.clone()),
            op = op_str,
            r = print_base(rhs.clone())
        )
    }

    pub fn display_time(&self, time: TimeIdx) -> String {
        let Time { event, offset } = self.get(time);
        format!("{event} + {}", self.display_expr(*offset))
    }

    fn display_time_sub(&self, ts: TimeSub) -> String {
        match ts {
            TimeSub::Unit(e) => self.display_expr(e),
            TimeSub::Sym { l, r } => {
                format!("({} - {})", self.display_time(l), self.display_time(r))
            }
        }
    }

    /// Display a proposition by recursively displaying its subexpressions.
    pub fn display_prop(&self, prop: PropIdx) -> String {
        match self.get(prop) {
            Prop::True => "true".to_string(),
            Prop::False => "false".to_string(),
            Prop::Cmp(c) => self.display_cmp(c, |e| self.display_expr(e)),
            Prop::TimeCmp(cmp) => {
                self.display_cmp(cmp, |t| self.display_time(t))
            }
            Prop::TimeSubCmp(cmp) => {
                self.display_cmp(cmp, |t| self.display_time_sub(t))
            }
            Prop::Not(p) => format!("!{}", self.display_prop(*p)),
            Prop::And(l, r) => format!(
                "({} & {})",
                self.display_prop(*l),
                self.display_prop(*r)
            ),
            Prop::Or(l, r) => format!(
                "({} | {})",
                self.display_prop(*l),
                self.display_prop(*r)
            ),
            Prop::Implies(l, r) => format!(
                "({} => {})",
                self.display_prop(*l),
                self.display_prop(*r)
            ),
        }
    }
}

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

// We can use indexing syntax for all values in the context for which it is a Ctx.
impl<K> std::ops::Index<Idx<K>> for Component
where
    Component: Ctx<K>,
{
    type Output = K;

    fn index(&self, index: Idx<K>) -> &Self::Output {
        self.get(index)
    }
}
