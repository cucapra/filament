use std::{collections::HashMap, fmt::Display};

use crate::errors;

use super::{Constraint, Id, TimeRep};

/// A range over time representation
#[derive(Clone)]
pub struct Range<T>
where
    T: TimeRep + Clone,
{
    pub start: T,
    pub end: T,
    pos: Option<errors::Span>,
}

impl<T> Range<T>
where
    T: TimeRep + Clone + PartialOrd,
{
    /// Generate constraints for well formedness of this range.
    pub fn well_formed(&self) -> impl Iterator<Item = Constraint<T>> {
        std::iter::once(
            Constraint::lt(self.start.clone(), self.end.clone()).add_note(
                "Interval's end time must be greater than the start time",
                self.pos.clone(),
            ),
        )
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
            pos: None,
        }
    }

    pub fn resolve(&self, bindings: &HashMap<Id, &T>) -> Self {
        Range {
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
            pos: self.pos.clone(),
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
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
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
#[derive(Clone)]
pub struct Interval<T>
where
    T: TimeRep + Clone,
{
    pub within: Range<T>,
    pub exact: Option<Range<T>>,
    pos: Option<errors::Span>,
}
impl<T> Interval<T>
where
    T: super::TimeRep + Clone,
{
    pub fn new(within: Range<T>, exact: Option<Range<T>>) -> Self {
        Self {
            within,
            exact,
            pos: None,
        }
    }

    pub fn with_exact(mut self, exact: Range<T>) -> Self {
        self.exact = Some(exact);
        self
    }

    pub fn resolve(&self, bindings: &HashMap<Id, &T>) -> Self {
        Interval {
            within: self.within.resolve(bindings),
            exact: self.exact.as_ref().map(|range| range.resolve(bindings)),
            pos: None,
        }
    }

    pub fn events(&self) -> Vec<&T> {
        let mut within = vec![&self.within.start, &self.within.end];
        if let Some(range) = &self.exact {
            within.append(&mut vec![&range.start, &range.end]);
        }
        within
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
                    .map(|con| {
                        con.add_note(
                            "@exact must be a subset of total interval",
                            self.pos.clone(),
                        )
                    })
                    .chain(ex.well_formed())
            })
            .chain(self.within.well_formed())
            .collect()
    }
}
impl<T> errors::WithPos for Interval<T>
where
    T: super::TimeRep,
{
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
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
            pos: None,
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
