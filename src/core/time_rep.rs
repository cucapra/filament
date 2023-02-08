use crate::interval_checking::SExp;

use super::{Id, Time};
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

    /// A time expression with exactly one event and offset
    fn unit(event: Id, state: u64) -> Self;
    /// Increment the time unit by a constant
    fn increment(self, n: u64) -> Self;
    /// Resolve the time expression given a binding
    fn resolve(&self, bindings: &Binding<Self>) -> Self;
    /// Substract two time expression representing the absolute difference
    fn sub(self, other: Self) -> Self::SubRep;
    /// All events used in the time expression
    fn events(&self) -> Vec<Id>;
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
pub enum TimeSub<T>
where
    T: TimeRep,
{
    /// Concrete difference between two time expressions
    Unit(u64),
    /// Symbolic difference between two time expressions
    Sym { l: T, r: T },
}

impl<T> TimeSub<T>
where
    T: TimeRep,
{
    pub fn unit(n: u64) -> Self {
        TimeSub::Unit(n)
    }

    pub fn sym(l: T, r: T) -> Self {
        TimeSub::Sym { l, r }
    }

    pub fn events(&self) -> Vec<Id> {
        match self {
            TimeSub::Unit(_) => vec![],
            TimeSub::Sym { l, r } => {
                let mut events = l.events();
                events.extend(r.events());
                events
            }
        }
    }

    /// Return the concrete difference if possible
    pub fn concrete(&self) -> Option<u64> {
        match self {
            TimeSub::Unit(n) => Some(*n),
            TimeSub::Sym { .. } => None,
        }
    }
}

impl<T> WithTime<T> for TimeSub<T>
where
    T: TimeRep,
{
    fn resolve(&self, bindings: &Binding<T>) -> Self {
        match self {
            TimeSub::Unit(n) => TimeSub::Unit(*n),
            TimeSub::Sym { l, r } => TimeSub::Sym {
                l: l.resolve(bindings),
                r: r.resolve(bindings),
            },
        }
    }
}

impl<T: Display + TimeRep> Display for TimeSub<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeSub::Unit(n) => write!(f, "{}", n),
            TimeSub::Sym { l, r } => write!(f, "|{} - {}|", l, r),
        }
    }
}

impl From<&TimeSub<Time<u64>>> for SExp {
    fn from(ts: &TimeSub<Time<u64>>) -> Self {
        match ts {
            TimeSub::Unit(n) => SExp(n.to_string()),
            TimeSub::Sym { l, r } => {
                SExp(format!("(abs (- {} {}))", SExp::from(l), SExp::from(r)))
            }
        }
    }
}
