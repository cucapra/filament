use std::fmt::Display;

use crate::{errors, utils::GPosIdx};

use super::{Binding, Constraint, OrderConstraint, TimeRep, WithTime};

/// A range over time representation
#[derive(Clone)]
pub struct Range<T>
where
    T: TimeRep + Clone,
{
    pub start: T,
    pub end: T,
    pos: GPosIdx,
}

impl<T> Range<T>
where
    T: TimeRep,
{
    /// Generate constraints for well formedness of this range.
    pub fn well_formed(&self) -> impl Iterator<Item = Constraint<T>> {
        std::iter::once(
            Constraint::from(OrderConstraint::lt(
                self.start.clone(),
                self.end.clone(),
            ))
            .add_note(
                "Interval's end time must be greater than the start time",
                self.pos,
            ),
        )
    }

    pub fn events(&self) -> Vec<&T> {
        vec![&self.start, &self.end]
    }

    /// Return the length of this range
    pub fn len(&self) -> T::SubRep {
        self.end.clone().sub(self.start.clone())
    }
}

impl<T> WithTime<T> for Range<T>
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &Binding<T>) -> Self {
        Range {
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
            ..self.clone()
        }
    }
}

impl<T> Range<T>
where
    T: TimeRep + Clone,
{
    pub fn new(start: T, end: T) -> Self {
        Self {
            start,
            end,
            pos: GPosIdx::UNKNOWN,
        }
    }
}

impl<T: TimeRep + PartialEq> PartialEq for Range<T> {
    fn eq(&self, other: &Self) -> bool {
        self.start == other.start && self.end == other.end
    }
}

impl<T> errors::WithPos for Range<T>
where
    T: TimeRep,
{
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

impl<T> Display for Range<T>
where
    T: Display + TimeRep + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@[{}, {}]", self.start, self.end)
    }
}
