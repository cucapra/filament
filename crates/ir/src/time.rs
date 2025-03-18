use crate::{DisplayCtx, MutCtx, Param, Prop};

use super::{
    AddCtx, Component, Ctx, EventIdx, Expr, ExprIdx, Foldable, ParamIdx,
    TimeIdx,
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
        C: Ctx<Time> + AddCtx<Expr>,
    {
        let l = ctx.get(self);
        let r = ctx.get(other);
        if l.event == r.event {
            TimeSub::Unit(l.offset.sub(r.offset, ctx))
        } else {
            TimeSub::Sym { l: self, r: other }
        }
    }

    /// Add a [TimeSub] to a time expression
    pub fn add(self, ts: &TimeSub, ctx: &mut Component) -> TimeIdx {
        match ts {
            TimeSub::Unit(e) => {
                let Time { event, offset } = ctx.get(self).clone();
                let offset = offset.add(*e, ctx);
                ctx.add(Time { event, offset })
            }
            TimeSub::Sym { .. } => {
                todo!(
                    "Cannot add `{}' and `{}'. Please report this as a bug with the program that triggered it.",
                    ctx.display(self),
                    ctx.display(ts)
                );
            }
        }
    }

    /// Get the event associated with the time expression
    pub fn event(self, ctx: &impl Ctx<Time>) -> EventIdx {
        let time = ctx.get(self);
        time.event
    }

    pub fn relevant_vars<C>(self, ctx: &C) -> Vec<ParamIdx>
    where
        C: Ctx<Time> + Ctx<Expr> + Ctx<Prop>,
    {
        let time = ctx.get(self);
        time.offset.relevant_vars(ctx)
    }

    pub fn valid<C>(&self, ctx: &C) -> bool
    where
        C: Ctx<Time> + Ctx<Expr> + Ctx<Prop> + MutCtx<Param>,
    {
        let time = ctx.get(*self);
        time.offset.valid(ctx)
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

impl Foldable<EventIdx, TimeIdx> for TimeSub {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(EventIdx) -> Option<TimeIdx>,
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
