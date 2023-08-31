use super::{idxs::PropIdx, AddCtx, Ctx, ExprIdx, InfoIdx, TimeIdx, TimeSub};
use crate::construct_binop;
use std::fmt::{self, Display};

#[derive(Clone, PartialEq, Eq, Hash)]
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
            Cmp::Eq => "=",
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
}

#[derive(Clone, PartialEq, Eq, Hash)]
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
