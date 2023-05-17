use super::{
    Event, Expr, ExprIdx, Indexed, Param, Port, Range, SmallIndexed, Time,
    TimeIdx,
};
use crate::utils::Idx;

pub struct Component {
    // Component defined values. Once created, we don't expect too many of these
    // to be created.
    /// Ports defined by the component
    ports: SmallIndexed<Port>,
    /// Parameters defined the component
    params: SmallIndexed<Param>,
    /// Events defined by the component
    events: SmallIndexed<Event>,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    exprs: Indexed<Expr>,
    /// Interned times
    times: Indexed<Time>,
    /// Interned ranges
    ranges: Indexed<Range>,
}

/// A context for interning values.
/// In addition to adding and getting values, the context also supports applying
/// a substitution to a value.
pub trait Ctx<T> {
    /// Add a value to the context
    fn add(&mut self, val: T) -> Idx<T>;
    /// Get the information associated with a value
    fn get(&self, idx: Idx<T>) -> &T;
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
