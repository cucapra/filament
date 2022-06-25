use std::{collections::HashMap, fmt::Display};

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
    T: TimeRep + Clone + PartialOrd,
{
    /// Generate constraints for well formedness of this range.
    pub fn well_formed(&self) -> impl Iterator<Item = Constraint<T>> {
        Constraint::lt(self.start.clone(), self.end.clone()).into_iter()
    }
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
}

impl<T> Display for Range<T>
where
    T: Display + TimeRep + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}, {}]", self.start, self.end)
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
    T: super::TimeRep + Clone,
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
}
impl<T> Interval<T>
where
    T: super::TimeRep + Clone + PartialOrd + PartialEq,
{
    /// Generate well formedness constraints for this interval
    pub fn well_formed(&self) -> Vec<Constraint<T>> {
        self.exact
            .iter()
            .flat_map(|ex| {
                Constraint::subset(ex.clone(), self.within.clone())
                    .chain(ex.well_formed())
            })
            .chain(self.within.well_formed())
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

impl<T> Display for Interval<T>
where
    T: Display + Clone + super::TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@{}", self.within)?;
        if let Some(interval) = &self.exact {
            write!(f, " + @exact{}", interval)?;
        }
        Ok(())
    }
}
