use super::{
    Ctx, Event, Expr, ExprIdx, Indexed, Param, Port, Range, SmallIndexed, Time,
    TimeIdx,
};
use crate::utils::Idx;

pub struct Component {
    // Component defined values. Once created, we don't expect too many of these
    // to be created.
    /// Ports and bundles defined by the component.
    ports: SmallIndexed<Port>,
    /// Parameters defined the component
    params: SmallIndexed<Param>,
    /// Events defined by the component
    events: SmallIndexed<Event>,

    /// Facts in the component.
    /// All nested facts are hoisted out to the top context by adding the path
    /// condition as the antecedant.
    // fact: SmallIndexed<Fact>,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    exprs: Indexed<Expr>,
    /// Interned times
    times: Indexed<Time>,
    /// Interned ranges
    ranges: Indexed<Range>,
}

impl Ctx<Expr> for Component {
    fn add(&mut self, val: Expr) -> ExprIdx {
        self.exprs.add(val)
    }

    fn get(&self, idx: ExprIdx) -> &Expr {
        self.exprs.get(idx)
    }
}

impl Ctx<Time> for Component {
    fn add(&mut self, val: Time) -> TimeIdx {
        self.times.add(val)
    }

    fn get(&self, idx: TimeIdx) -> &Time {
        self.times.get(idx)
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
