use super::Id;
use std::{collections::HashMap, fmt::Display};

/// A representation of time
pub trait TimeRep
where
    Self: Sized + Clone + Display,
{
    /// Representation of absolute difference b/w two events of this TimeRep
    type SubRep: WithTime<Self> + Clone + Display;

    fn unit(event: Id, state: u64) -> Self;
    fn increment(self, n: u64) -> Self;
    fn resolve(&self, bindings: &HashMap<Id, &Self>) -> Self;
    fn sub(self, other: Self) -> Self::SubRep;
}

/// Functions provided by data structures that contain a time representation
pub trait WithTime<T>
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &HashMap<Id, &T>) -> Self;
}

impl<T> WithTime<T> for T
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &HashMap<Id, &T>) -> Self {
        self.resolve(bindings)
    }
}

/// Represents the absolute difference between two time events
#[derive(Clone)]
pub struct TimeSub<T>
where
    T: TimeRep,
{
    pub a: T,
    pub b: T,
}

impl<T> WithTime<T> for TimeSub<T>
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &std::collections::HashMap<Id, &T>) -> Self {
        Self {
            a: self.a.resolve(bindings),
            b: self.b.resolve(bindings),
        }
    }
}

impl<T: TimeRep + Display> Display for TimeSub<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "|{} - {}|", self.a, self.b)
    }
}
