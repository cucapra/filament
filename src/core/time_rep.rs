use super::{ConcTime, Id, PortParam, Width};
use crate::utils::SExp;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::{fmt::Debug, fmt::Display};

#[derive(Default)]
/// Represents a binding from names to type T.
pub struct Binding<T> {
    map: LinkedHashMap<Id, T>,
}

impl<T> Binding<T> {
    pub fn new(map: impl IntoIterator<Item = (Id, T)>) -> Self {
        Self {
            map: map.into_iter().collect(),
        }
    }

    pub fn find(&self, n: &Id) -> Option<&T> {
        self.map.get(n)
    }

    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.map.values()
    }

    /// Return binding for n, or panic if it doesn't exist
    pub fn get(&self, n: &Id) -> &T {
        self.find(n).unwrap_or_else(|| panic!("No binding for {n}"))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Id, &T)> {
        self.map.iter()
    }

    pub fn extend(&mut self, other: Vec<(Id, T)>) {
        self.map.extend(other);
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<T> IntoIterator for Binding<T> {
    type Item = (Id, T);
    type IntoIter = linked_hash_map::IntoIter<Id, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<T> Debug for Binding<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.map.iter().map(|(k, v)| format!("{k}->{v}")).join(", ")
        )
    }
}

/// Represents the absolute difference between two time events
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TimeSub {
    /// Concrete difference between two time expressions
    Unit(PortParam),
    /// Symbolic difference between two time expressions
    Sym { l: ConcTime, r: ConcTime },
}

impl TimeSub {
    pub fn unit(n: PortParam) -> Self {
        TimeSub::Unit(n)
    }

    pub fn sym(l: ConcTime, r: ConcTime) -> Self {
        TimeSub::Sym { l, r }
    }

    pub fn concrete(&self) -> Option<u64> {
        match self {
            TimeSub::Unit(PortParam::Const(n)) => Some(*n),
            _ => None,
        }
    }
}

impl TimeSub {
    pub fn resolve_event(&self, bindings: &Binding<ConcTime>) -> Self {
        match self {
            // Unit cannot contain any events
            TimeSub::Unit(_) => self.clone(),
            TimeSub::Sym { l, r } => TimeSub::Sym {
                l: l.resolve_event(bindings),
                r: r.resolve_event(bindings),
            },
        }
    }

    pub fn resolve_offset(&self, bindings: &Binding<Width>) -> Self {
        match self {
            TimeSub::Unit(n) => TimeSub::Unit(n.resolve(bindings).unwrap()),
            TimeSub::Sym { l, r } => TimeSub::Sym {
                l: l.resolve_offset(bindings),
                r: r.resolve_offset(bindings),
            },
        }
    }

    pub fn events(&self) -> Vec<Id> {
        match self {
            TimeSub::Unit(_) => vec![],
            TimeSub::Sym { l, r } => {
                vec![l.event(), r.event()]
            }
        }
    }
}

impl Display for TimeSub {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeSub::Unit(n) => write!(f, "{}", n),
            TimeSub::Sym { l, r } => write!(f, "|{} - {}|", l, r),
        }
    }
}

impl From<TimeSub> for SExp {
    fn from(ts: TimeSub) -> Self {
        match ts {
            TimeSub::Unit(n) => SExp(n.to_string()),
            TimeSub::Sym { l, r } => {
                SExp(format!("(abs (- {} {}))", SExp::from(l), SExp::from(r)))
            }
        }
    }
}

impl From<PortParam> for TimeSub {
    fn from(n: PortParam) -> Self {
        TimeSub::Unit(n)
    }
}
