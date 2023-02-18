use super::{Constraint, Expr, OrderConstraint, Time, TimeSub};
use crate::diagnostics::Diagnostics;
use crate::utils::Binding;
use crate::{errors, utils::GPosIdx};
use derivative::Derivative;
use std::fmt::Display;

/// A range over time representation
#[derive(Clone, Derivative)]
#[derivative(PartialEq)]
pub struct Range {
    pub start: Time,
    pub end: Time,
    #[derivative(PartialEq = "ignore")]
    pos: GPosIdx,
}

impl Range {
    /// Generate constraints for well formedness of this range.
    pub fn well_formed(&self, diag: &mut Diagnostics) -> Constraint {
        Constraint::base(OrderConstraint::lt(
            self.start.clone(),
            self.end.clone(),
        ))
        .add_note(diag.add_info(
            "interval's end time must be greater than the start time",
            self.pos,
        ))
    }

    /// Length of this range
    pub fn len(&self) -> TimeSub {
        self.end.clone() - self.start.clone()
    }

    /// Resolve events mentioned in this range
    pub fn resolve_event(&self, bindings: &Binding<Time>) -> Self {
        Range {
            start: self.start.resolve_event(bindings),
            end: self.end.resolve_event(bindings),
            ..self.clone()
        }
    }

    /// Resolve [Expr] mentioned in this range.
    pub fn resolve_exprs(&self, bindings: &Binding<Expr>) -> Self {
        Range {
            start: self.start.resolve_expr(bindings),
            end: self.end.resolve_expr(bindings),
            ..self.clone()
        }
    }

    /// Returns all the time expressions associated with this range
    pub fn time_exprs(&self) -> Vec<&Time> {
        vec![&self.start, &self.end]
    }
}

impl Range {
    pub fn new(start: Time, end: Time) -> Self {
        Self {
            start,
            end,
            pos: GPosIdx::UNKNOWN,
        }
    }
}

impl errors::WithPos for Range {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

impl Display for Range {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@[{}, {}]", self.start, self.end)
    }
}
