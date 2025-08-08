use super::{
    AddCtx, Ctx, ExprIdx, InfoIdx, MutCtx, TimeIdx, TimeSub, idxs::PropIdx,
};
use crate::{Event, EventIdx, Expr, Param, ParamIdx, Time, construct_binop};
use std::fmt::{self, Display};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
/// Comparison operators
pub enum Cmp {
    Gt,
    Gte,
    Eq,
}

impl fmt::Display for Cmp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op = match self {
            Cmp::Gt => ">",
            Cmp::Eq => "==",
            Cmp::Gte => ">=",
        };
        write!(f, "{}", op)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
/// Comparison between two expressions of type T
pub struct CmpOp<T> {
    pub op: Cmp,
    pub lhs: T,
    pub rhs: T,
}

impl<T> CmpOp<T> {
    pub fn gt(lhs: T, rhs: T) -> Self {
        Self {
            op: Cmp::Gt,
            lhs,
            rhs,
        }
    }

    pub fn gte(lhs: T, rhs: T) -> Self {
        Self {
            op: Cmp::Gte,
            lhs,
            rhs,
        }
    }

    pub fn eq(lhs: T, rhs: T) -> Self {
        Self {
            op: Cmp::Eq,
            lhs,
            rhs,
        }
    }

    pub fn lt(lhs: T, rhs: T) -> Self {
        Self {
            op: Cmp::Gt,
            lhs: rhs,
            rhs: lhs,
        }
    }

    pub fn lte(lhs: T, rhs: T) -> Self {
        Self {
            op: Cmp::Gte,
            lhs: rhs,
            rhs: lhs,
        }
    }
}

impl<T: Display> fmt::Display for CmpOp<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.lhs, self.op, self.rhs)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
/// A proposition
pub enum Prop {
    True,
    False,
    /// A comparison between two expressions
    Cmp(CmpOp<ExprIdx>),
    /// Comparison between time expressions
    TimeCmp(CmpOp<TimeIdx>),
    /// Comparison between time-sub expressions
    TimeSubCmp(CmpOp<TimeSub>),
    Not(PropIdx),
    And(PropIdx, PropIdx),
    Or(PropIdx, PropIdx),
    Implies(PropIdx, PropIdx),
}

impl fmt::Display for Prop {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Prop::True => write!(f, "true"),
            Prop::False => write!(f, "false"),
            Prop::Cmp(cmp) => write!(f, "{cmp}"),
            Prop::TimeCmp(cmp) => write!(f, "{cmp}"),
            Prop::TimeSubCmp(cmp) => write!(f, "{cmp}"),
            Prop::Not(p) => write!(f, "!{p}"),
            Prop::And(l, r) => write!(f, "{l} & {r}"),
            Prop::Or(l, r) => write!(f, "{l} | {r}"),
            Prop::Implies(l, r) => write!(f, "{l} => {r}"),
        }
    }
}

/// A more elaborated version of comparison operators used to generate CmpOps.
enum RawCmp {
    Gt,
    Gte,
    Eq,
    Lt,
    Lte,
}

impl RawCmp {
    /// Converts a [RawCmp] to a [CmpOp].
    pub fn cmp_op<T>(&self, lhs: T, rhs: T) -> CmpOp<T> {
        match self {
            RawCmp::Gt => CmpOp::gt(lhs, rhs),
            RawCmp::Gte => CmpOp::gte(lhs, rhs),
            RawCmp::Eq => CmpOp::eq(lhs, rhs),
            RawCmp::Lt => CmpOp::lt(lhs, rhs),
            RawCmp::Lte => CmpOp::lte(lhs, rhs),
        }
    }
}

impl Prop {
    /// Creates a [Prop::Cmp] from two [ExprIdx]s and a [Cmp].
    fn cmp(lhs: ExprIdx, rhs: ExprIdx, op: RawCmp) -> Self {
        Self::Cmp(op.cmp_op(lhs, rhs))
    }

    /// Creates a [Prop::TimeCmp] from two [TimeIdx]s and a [Cmp].
    fn timecmp(lhs: TimeIdx, rhs: TimeIdx, op: RawCmp) -> Self {
        Self::TimeCmp(op.cmp_op(lhs, rhs))
    }

    /// Creates a [Prop::TimeSubCmp] from two [TimeSub]s and a [Cmp].
    fn timesubcmp(lhs: TimeSub, rhs: TimeSub, op: RawCmp) -> Self {
        Self::TimeSubCmp(op.cmp_op(lhs, rhs))
    }
}

// helper macro for generating propositions from comparable elements
macro_rules! construct_props {
    (<$ctx: ty>($cmp: expr, $in: ty)) => {
        construct_binop!(
            <$ctx>($cmp, $in) => Prop;
            gt = RawCmp::Gt;
            gte = RawCmp::Gte;
            equal = RawCmp::Eq;
            lt = RawCmp::Lt;
            lte = RawCmp::Lte;
        );
    }

}
// creates the expression comparison constructors for propositions
construct_props!(<impl AddCtx<Prop>>(Prop::cmp, ExprIdx));
// creates the time comparison constructors for propositions
construct_props!(<impl AddCtx<Prop>>(Prop::timecmp, TimeIdx));
// creates the timesub comparison constructors for propositions
construct_props!(<impl AddCtx<Prop>>(Prop::timesubcmp, TimeSub));

/// Constructors for propositions
impl PropIdx {
    #[inline(always)]
    /// Returns true of this proposition is definitely true.
    /// This is a purely syntactic check, and does not attempt to reduce the
    /// proposition.
    pub fn is_true(&self, ctx: &impl Ctx<Prop>) -> bool {
        matches!(ctx.get(*self), Prop::True)
    }

    #[inline(always)]
    /// Returns true of this proposition is definitely false.
    /// This is a purely syntactic check, and does not attempt to reduce the
    /// proposition.
    pub fn is_false(&self, ctx: &impl Ctx<Prop>) -> bool {
        matches!(ctx.get(*self), Prop::False)
    }

    pub fn as_concrete(&self, ctx: &impl Ctx<Prop>) -> Option<bool> {
        match ctx.get(*self) {
            Prop::True => Some(true),
            Prop::False => Some(false),
            _ => None,
        }
    }

    /// Negation of a proposition
    pub fn not(self, ctx: &mut impl AddCtx<Prop>) -> PropIdx {
        ctx.add(Prop::Not(self))
    }

    /// Conjunction of two propositions
    pub fn and(self, other: PropIdx, ctx: &mut impl AddCtx<Prop>) -> PropIdx {
        ctx.add(Prop::And(self, other))
    }

    /// Disjunction of two propositions
    pub fn or(self, other: PropIdx, ctx: &mut impl AddCtx<Prop>) -> PropIdx {
        ctx.add(Prop::Or(self, other))
    }

    /// Implication from a proposition to another
    pub fn implies(
        self,
        cons: PropIdx,
        ctx: &mut impl AddCtx<Prop>,
    ) -> PropIdx {
        ctx.add(Prop::Implies(self, cons))
    }
}

/// Queries over propositions
impl PropIdx {
    /// Returns the consequent of an implication, if this proposition is an
    /// implication. Otherwise, returns the proposition itself.
    pub fn consequent(self, ctx: &impl Ctx<Prop>) -> PropIdx {
        match ctx.get(self) {
            Prop::Implies(_, cons) => *cons,
            _ => self,
        }
    }

    // relevant vars of an if guard -- can't include times
    pub fn relevant_vars_if_acc<C>(
        &self,
        ctx: &C,
        param_acc: &mut Vec<ParamIdx>,
    ) where
        C: Ctx<Prop> + Ctx<Expr>,
    {
        match ctx.get(*self) {
            Prop::True | Prop::False => (),
            Prop::Cmp(CmpOp { lhs, rhs, .. }) => {
                lhs.relevant_vars_acc(ctx, param_acc);
                rhs.relevant_vars_acc(ctx, param_acc);
            }
            Prop::TimeCmp(_) | Prop::TimeSubCmp(_) => {
                panic!("if-expr guards cannot contain times");
            }
            Prop::Not(p) => p.relevant_vars_if_acc(ctx, param_acc),
            Prop::And(l, r) | Prop::Or(l, r) | Prop::Implies(l, r) => {
                l.relevant_vars_if_acc(ctx, param_acc);
                r.relevant_vars_if_acc(ctx, param_acc)
            }
        }
    }

    /// Accumulate all the parameters and events that appear in this proposition.
    pub fn relevant_vars_acc<C>(
        &self,
        ctx: &C,
        param_acc: &mut Vec<ParamIdx>,
        event_acc: &mut Vec<EventIdx>,
    ) where
        C: Ctx<Prop> + Ctx<Time> + Ctx<Expr>,
    {
        let mut time_acc = |time: TimeIdx, params: &mut Vec<ParamIdx>| {
            let Time { event, offset } = ctx.get(time);
            event_acc.push(*event);
            offset.relevant_vars_acc(ctx, params);
        };

        let mut time_sub_acc = |ts: &TimeSub| match ts {
            TimeSub::Unit(e) => e.relevant_vars_acc(ctx, param_acc),
            TimeSub::Sym { l, r } => {
                time_acc(*l, param_acc);
                time_acc(*r, param_acc);
            }
        };

        match ctx.get(*self) {
            Prop::True | Prop::False => (),
            Prop::Cmp(CmpOp { lhs, rhs, .. }) => {
                lhs.relevant_vars_acc(ctx, param_acc);
                rhs.relevant_vars_acc(ctx, param_acc);
            }
            Prop::TimeCmp(CmpOp { lhs, rhs, .. }) => {
                time_acc(*lhs, param_acc);
                time_acc(*rhs, param_acc);
            }
            Prop::TimeSubCmp(CmpOp { lhs, rhs, .. }) => {
                time_sub_acc(lhs);
                time_sub_acc(rhs);
            }
            Prop::Not(p) => {
                p.relevant_vars_acc(ctx, param_acc, event_acc);
            }
            Prop::And(l, r) | Prop::Or(l, r) | Prop::Implies(l, r) => {
                l.relevant_vars_acc(ctx, param_acc, event_acc);
                r.relevant_vars_acc(ctx, param_acc, event_acc);
            }
        }
    }

    /// Returns the parameters and events mentioned in the proposition.
    #[inline]
    pub fn relevant_vars<C>(&self, ctx: &C) -> (Vec<ParamIdx>, Vec<EventIdx>)
    where
        C: Ctx<Time> + Ctx<Prop> + Ctx<Expr>,
    {
        let mut params = Vec::new();
        let mut events = Vec::new();
        self.relevant_vars_acc(ctx, &mut params, &mut events);
        (params, events)
    }

    pub fn relevant_vars_if<C>(&self, ctx: &C) -> Vec<ParamIdx>
    where
        C: Ctx<Prop> + Ctx<Expr>,
    {
        let mut params = Vec::new();
        self.relevant_vars_if_acc(ctx, &mut params);
        params
    }

    pub fn relevant_props<C>(&self, ctx: &C) -> Vec<PropIdx>
    where
        C: Ctx<Prop> + Ctx<Expr>,
    {
        let mut props = Vec::new();
        self.relevant_props_acc(ctx, &mut props);
        props
    }

    pub fn relevant_props_acc<C>(&self, ctx: &C, props: &mut Vec<PropIdx>)
    where
        C: Ctx<Prop> + Ctx<Expr>,
    {
        match ctx.get(*self) {
            Prop::True | Prop::False => (),
            Prop::Cmp(CmpOp { lhs, rhs, .. }) => {
                let lhs_props = lhs.relevant_props(ctx);
                let rhs_props = rhs.relevant_props(ctx);
                props.extend(lhs_props);
                props.extend(rhs_props);
            }
            Prop::TimeCmp(_) => todo!(),
            Prop::TimeSubCmp(_) => todo!(),
            Prop::Not(p) => {
                let inner_props = p.relevant_props(ctx);
                props.extend(inner_props);
            }
            Prop::And(l, r) | Prop::Or(l, r) | Prop::Implies(l, r) => {
                let l_props = l.relevant_props(ctx);
                let r_props = r.relevant_props(ctx);
                props.extend(l_props);
                props.extend(r_props);
            }
        }
    }

    pub fn valid<C>(&self, ctx: &C) -> bool
    where
        C: Ctx<Prop> + Ctx<Time> + Ctx<Expr> + MutCtx<Param> + MutCtx<Event>,
    {
        let (props, events) = self.relevant_vars(ctx);
        props.iter().all(|p| ctx.valid(*p))
            && events.iter().all(|e| ctx.valid(*e))
    }
}

#[derive(Clone, PartialEq, Eq)]
/// A fact in the program.
/// If `checked` is true, then this represents an assertion that needs to be
/// checked. Otherwise, it is an assumption.
pub struct Fact {
    pub prop: PropIdx,
    pub reason: InfoIdx,
    pub(super) checked: bool,
}

impl Fact {
    /// An assertion which is required to be statically proved
    pub fn assert(prop: PropIdx, reason: InfoIdx) -> Self {
        Self {
            prop,
            reason,
            checked: true,
        }
    }

    /// An assumption which is not checked
    /// Outside the IR library, use [Component::assume] instead.
    pub(super) fn assume(prop: PropIdx, reason: InfoIdx) -> Self {
        Self {
            prop,
            reason,
            checked: false,
        }
    }

    /// Is this an assumption?
    pub fn is_assume(&self) -> bool {
        !self.checked
    }

    /// Is this an assertion?
    pub fn is_assert(&self) -> bool {
        self.checked
    }
}
