use super::{Binding, Constraint, Expr, OrderConstraint, Time, TimeSub};
use std::fmt::Display;

/// A range over time representation
#[derive(Clone)]
pub struct Range {
    pub start: Time,
    pub end: Time,
}

impl Range {
    /// Generate constraints for well formedness of this range.
    pub fn well_formed(&self) -> Constraint {
        Constraint::base(OrderConstraint::lt(
            self.start.clone(),
            self.end.clone(),
        ))
    }

    /// Length of this range
    pub fn len(&self) -> TimeSub {
        self.end.clone() - self.start.clone()
    }

    /// Resolve events mentioned in this range
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        Range {
            start: self.start.resolve_event(bindings),
            end: self.end.resolve_event(bindings),
        }
    }

    /// Resolve [Expr] mentioned in this range.
    pub fn resolve_exprs(self, bindings: &Binding<Expr>) -> Self {
        Range {
            start: self.start.resolve_expr(bindings),
            end: self.end.resolve_expr(bindings),
        }
    }

    /// Returns all the time expressions associated with this range
    pub fn time_exprs(&self) -> Vec<&Time> {
        vec![&self.start, &self.end]
    }
}

impl Range {
    pub fn new(start: Time, end: Time) -> Self {
        Self { start, end }
    }
}

impl Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@[{}, {}]", self.start, self.end)
    }
}
