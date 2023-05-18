use super::{Ctx, EventIdx, Expr, ExprIdx, TimeIdx};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// A temporal event. Represents an offset from the start of the event.
pub struct Time {
    pub event: EventIdx,
    pub offset: ExprIdx,
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
