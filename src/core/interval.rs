use std::collections::HashMap;

use super::{Id, TimeRep};

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
    pub fn resolve(&self, bindings: &HashMap<Id, T>) -> Self {
        Range {
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
        }
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
    T: super::TimeRep + Clone,
{
    pub fn new(within: Range<T>) -> Self {
        Interval {
            within,
            exact: None,
        }
    }

    pub fn with_exact(mut self, exact: Range<T>) -> Self {
        self.exact = Some(exact);
        self
    }

    pub fn resolve(&self, bindings: &HashMap<Id, T>) -> Self {
        Interval {
            within: self.within.resolve(bindings),
            exact: self.exact.as_ref().map(|range| range.resolve(bindings)),
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
