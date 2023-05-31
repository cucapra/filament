use super::{
    Component, Ctx, EventIdx, Expr, ExprIdx, Foldable, ParamIdx, TimeIdx,
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
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Represents the difference between two events.
pub enum TimeSub {
    /// Concrete difference between two time expressions
    Unit(ExprIdx),
    /// Symbolic difference between two time expressions
    Sym { l: TimeIdx, r: TimeIdx },
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
