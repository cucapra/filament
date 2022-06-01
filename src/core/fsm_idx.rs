use itertools::Itertools;

use crate::interval_checking::SExp;

use super::{Id, Interval};

/// Represents a state in an FSM. Corresponds to expressions like T+n
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct FsmIdx {
    pub name: Id,
    pub state: u64,
}
impl std::fmt::Debug for FsmIdx {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.state == 0 {
            write!(f, "{}", self.name)
        } else {
            write!(f, "{}+{}", self.name, self.state)
        }
    }
}

impl FsmIdx {
    fn new(name: Id, state: u64) -> Self {
        Self { name, state }
    }
}

/// An interval time expression that denotes a max of sums expression.
#[derive(Default, Hash, Clone, PartialEq, Eq)]
pub struct FsmIdxs {
    pub fsms: Vec<FsmIdx>,
}

impl std::fmt::Debug for FsmIdxs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.fsms.len() == 1 {
            write!(f, "{fsm:?}", fsm = &self.fsms[0])
        } else {
            write!(f, "max(")?;
            write!(
                f,
                "{}",
                self.fsms.iter().map(|fsm| format!("{fsm:?}")).join(",")
            )?;
            write!(f, ")")
        }
    }
}

impl FsmIdxs {
    /// Construct an index with exactly one FSM
    pub fn unit(name: Id, state: u64) -> Self {
        FsmIdxs {
            fsms: vec![FsmIdx { name, state }],
        }
    }

    /// Add a new fsm state T+n to the expression
    pub fn insert(&mut self, name: Id, state: u64) {
        self.fsms.push(FsmIdx::new(name, state))
    }

    /// Return the names of all events used in the max expression
    pub fn events(&self) -> impl Iterator<Item = &Id> {
        self.fsms.iter().map(|idx| &idx.name)
    }

    /// Increment all the the FSM states by the provided value
    pub fn increment(self, n: u64) -> Self {
        let fsms = self
            .fsms
            .into_iter()
            .map(|FsmIdx { name, state }| FsmIdx::new(name, state + n))
            .collect();
        FsmIdxs { fsms }
    }
}

impl super::TimeRep for FsmIdxs {
    fn resolve(&self, bindings: &std::collections::HashMap<Id, Self>) -> Self {
        let mut out = FsmIdxs::default();
        for FsmIdx { name, state } in &self.fsms {
            let mut idxs = bindings
                .get(name)
                .unwrap_or_else(|| panic!("No binding for {}", name))
                .clone()
                .increment(*state);
            out.fsms.append(&mut idxs.fsms);
        }
        out
    }
}

impl From<&FsmIdxs> for SExp {
    fn from(idxs: &FsmIdxs) -> Self {
        idxs.fsms
            .iter()
            .map(|FsmIdx { name, state }| {
                SExp(if *state == 0 {
                    format!("{name}")
                } else {
                    format!("(+ {name} {state})")
                })
            })
            .reduce(|acc, fsm| SExp(format!("(max {} {})", acc, fsm)))
            .unwrap()
    }
}

impl Interval<FsmIdxs> {
    /// Attempts to convert interval into (event, start, end). Only possible
    /// when the interval uses exactly the one event for both start and end.
    pub fn as_offset(&self) -> Option<(&Id, u64, u64)> {
        if self.start.fsms.len() != 1
            || self.end.fsms.len() != 1
            || self.start.fsms[0].name != self.end.fsms[0].name
        {
            None
        } else {
            Some((
                &self.start.fsms[0].name,
                self.start.fsms[0].state,
                self.end.fsms[0].state,
            ))
        }
    }
}
