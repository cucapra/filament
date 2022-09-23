use super::Id;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::{fmt::Debug, fmt::Display};

/// Represents a binding from names to time variables.
pub struct Binding<T>
where
    T: TimeRep,
{
    map: LinkedHashMap<Id, T>,
}

impl<T> Binding<T>
where
    T: TimeRep,
{
    pub fn new(map: LinkedHashMap<Id, T>) -> Self {
        Self { map }
    }

    pub fn find(&self, n: &Id) -> Option<&T> {
        self.map.get(n)
    }

    pub fn get(&self, n: &Id) -> &T {
        self.find(n).unwrap_or_else(|| panic!("No binding for {n}"))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &T)> {
        self.map.iter()
    }

    pub fn extend(&mut self, other: Vec<(Id, T)>) {
        self.map.extend(other);
    }
}

impl<T> Debug for Binding<T>
where
    T: Display + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.map.iter().map(|(k, v)| format!("{k}->{v}")).join(", ")
        )
    }
}

/// A representation of time
pub trait TimeRep
where
    Self: Sized + Clone + Display,
{
    /// Representation of absolute difference b/w two events of this TimeRep
    type SubRep: WithTime<Self> + Clone + Display;

    fn unit(event: Id, state: u64) -> Self;
    fn increment(self, n: u64) -> Self;
    fn resolve(&self, bindings: &Binding<Self>) -> Self;
    fn sub(self, other: Self) -> Self::SubRep;
}

/// Functions provided by data structures that contain a time representation
pub trait WithTime<T>
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &Binding<T>) -> Self;
}

impl<T> WithTime<T> for T
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &Binding<T>) -> Self {
        self.resolve(bindings)
    }
}

/// Represents the absolute difference between two time events
#[derive(Clone, PartialEq, Eq, Hash)]
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
    fn resolve(&self, bindings: &Binding<T>) -> Self {
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
