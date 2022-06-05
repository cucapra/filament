use std::{collections::HashMap, iter};

use super::{Constraint, Id, TimeRep};

/// A range over time representation
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Range<T>
where
    T: TimeRep + Clone,
{
    pub start: T,
    pub end: T,
}

impl<T> Range<T>
where
    T: TimeRep + Clone,
{
    pub fn new(start: T, end: T) -> Self {
        Self { start, end }
    }

    pub fn resolve(&self, bindings: &HashMap<Id, &T>) -> Self {
        Range {
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
        }
    }

    /// Generate constraints for well formedness of this range.
    pub fn well_formed(&self) -> Constraint<T> {
        Constraint::lt(self.start.clone(), self.end.clone())
    }
}

impl<T> std::fmt::Debug for Range<T>
where
    T: TimeRep + Clone + std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}, {:?}]", self.start, self.end)
    }
}

/// An interval consists of a type tag, a start time, and a end time.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Interval<T>
where
    T: TimeRep + Clone,
{
    pub within: Range<T>,
    pub exact: Option<Range<T>>,
}
impl<T> Interval<T>
where
    T: super::TimeRep + Clone + PartialEq,
{
    pub fn with_exact(mut self, exact: Range<T>) -> Self {
        self.exact = Some(exact);
        self
    }

    pub fn resolve(&self, bindings: &HashMap<Id, &T>) -> Self {
        Interval {
            within: self.within.resolve(bindings),
            exact: self.exact.as_ref().map(|range| range.resolve(bindings)),
        }
    }

    /// Generate well formedness constraints for this interval
    pub fn well_formed(&self) -> Vec<Constraint<T>> {
        self.exact
            .iter()
            .flat_map(|ex| {
                Constraint::subset(ex.clone(), self.within.clone())
                    .chain(iter::once(ex.well_formed()))
            })
            .chain(iter::once(self.within.well_formed()))
            .collect()
    }
}

impl<T> From<Range<T>> for Interval<T>
where
    T: super::TimeRep + Clone,
{
    fn from(within: Range<T>) -> Self {
        Interval {
            within,
            exact: None,
        }
    }
}

impl<T> std::fmt::Debug for Interval<T>
where
    T: std::fmt::Debug + Clone + super::TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{:?}", self.within)?;
        if let Some(interval) = &self.exact {
            write!(f, " + @exact{:?}", interval)?;
        }
        Ok(())
    }
}
