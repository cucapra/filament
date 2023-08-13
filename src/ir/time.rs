use super::{
    CmpOp, Component, Ctx, EventIdx, Expr, ExprIdx, Foldable, ParamIdx, Prop,
    PropIdx, TimeIdx,
};
use std::fmt::{self, Display};

#[derive(PartialEq, Eq, Hash, Clone)]
/// A temporal event. Represents an offset from the start of the event.
pub struct Time {
    pub event: EventIdx,
    pub offset: ExprIdx,
}

impl Display for Time {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}+{}", self.event, self.offset)
    }
}

impl TimeIdx {
    /// Construct a [TimeSub] by subtracting two time expressions
    pub fn sub<C>(self, other: TimeIdx, ctx: &mut C) -> TimeSub
    where
        C: Ctx<Time> + Ctx<Expr>,
    {
        let l = ctx.get(self);
        let r = ctx.get(other);
        if l.event == r.event {
            TimeSub::Unit(l.offset.sub(r.offset, ctx))
        } else {
            TimeSub::Sym { l: self, r: other }
        }
    }

    pub fn event(self, ctx: &impl Ctx<Time>) -> EventIdx {
        let time = ctx.get(self);
        time.event
    }

    pub fn lte<C>(self, other: TimeIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Time> + Ctx<Expr> + Ctx<Prop>,
    {
        ctx.add(Prop::TimeCmp(CmpOp::lte(self, other)))
    }

    pub fn lt<C>(self, other: TimeIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Time> + Ctx<Expr> + Ctx<Prop>,
    {
        ctx.add(Prop::TimeCmp(CmpOp::lt(self, other)))
    }

    pub fn gt<C>(self, other: TimeIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Time> + Ctx<Expr> + Ctx<Prop>,
    {
        ctx.add(Prop::TimeCmp(CmpOp::gt(self, other)))
    }

    pub fn gte<C>(self, other: TimeIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Time> + Ctx<Expr> + Ctx<Prop>,
    {
        ctx.add(Prop::TimeCmp(CmpOp::gte(self, other)))
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Represents the difference between two events.
pub enum TimeSub {
    /// Concrete difference between two time expressions
    Unit(ExprIdx),
    /// Symbolic difference between two time expressions
    Sym { l: TimeIdx, r: TimeIdx },
}

impl TimeSub {
    pub fn gt(self, other: TimeSub, ctx: &mut Component) -> PropIdx {
        ctx.add(Prop::TimeSubCmp(CmpOp::gt(self, other)))
    }

    pub fn gte(self, other: TimeSub, ctx: &mut Component) -> PropIdx {
        ctx.add(Prop::TimeSubCmp(CmpOp::gte(self, other)))
    }

    pub fn lt(self, other: TimeSub, ctx: &mut Component) -> PropIdx {
        ctx.add(Prop::TimeSubCmp(CmpOp::lt(self, other)))
    }

    pub fn lte(self, other: TimeSub, ctx: &mut Component) -> PropIdx {
        ctx.add(Prop::TimeSubCmp(CmpOp::lte(self, other)))
    }
}

impl From<ExprIdx> for TimeSub {
    fn from(e: ExprIdx) -> Self {
        TimeSub::Unit(e)
    }
}

impl Display for TimeSub {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimeSub::Unit(e) => write!(f, "{}", e),
            TimeSub::Sym { l, r } => write!(f, "{}-{}", l, r),
        }
    }
}

impl Foldable<ParamIdx, ExprIdx> for TimeSub {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(ParamIdx) -> Option<ExprIdx>,
    {
        match self {
            TimeSub::Unit(e) => TimeSub::Unit(e.fold_with(ctx, subst_fn)),
            TimeSub::Sym { l, r } => TimeSub::Sym {
                l: l.fold_with(ctx, subst_fn),
                r: r.fold_with(ctx, subst_fn),
            },
        }
    }
}
