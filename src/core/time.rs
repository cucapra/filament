use itertools::Itertools;

use super::{Expr, Id, Range};
use crate::utils::{self, SExp};
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash)]
/// An opaque time sum expression
/// Comparison between two [TimeSum] does not take into account the order of the
/// variables.
pub struct TimeSum {
    // Concrete part of the time sum
    concrete: u64,
    // Abstract part of the time sum
    abs: Vec<Id>,
}

impl PartialOrd for TimeSum {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let s1 = self.abs.iter().sorted().collect_vec();
        let s2 = other.abs.iter().sorted().collect_vec();
        if s1 == s2 {
            self.concrete.partial_cmp(&other.concrete)
        } else {
            None
        }
    }
}

impl TimeSum {
    pub fn concrete(&self) -> u64 {
        assert!(self.abs.is_empty());
        self.concrete
    }
}

impl From<Vec<u64>> for TimeSum {
    fn from(v: Vec<u64>) -> Self {
        Self {
            concrete: v.iter().sum(),
            abs: vec![],
        }
    }
}

impl From<Vec<Expr>> for TimeSum {
    fn from(v: Vec<Expr>) -> Self {
        let mut ts = Self {
            concrete: 0,
            abs: vec![],
        };
        for p in v {
            match p {
                Expr::Const(c) => ts.concrete += c,
                Expr::Var(v) => ts.abs.push(v),
            }
        }
        ts
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd)]
/// Represents expression of the form `G+1+k`
pub struct Time {
    /// The event for the time expression
    pub event: Id,
    /// The offsets for the time expression
    offset: TimeSum,
}

impl Time {
    pub fn new(event: Id, offset: Vec<Expr>) -> Self {
        Self {
            event,
            offset: TimeSum::from(offset),
        }
    }

    /// Get the offset associated with this time expression
    pub fn offset(&self) -> &TimeSum {
        &self.offset
    }

    /// Unit time expression that occurs when the event occurs
    pub fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: vec![Expr::Const(state)].into(),
        }
    }

    /// Time expression that occurs `n` cycles after this time expression
    pub fn increment(mut self, n: Expr) -> Self {
        match n {
            Expr::Const(n) => self.offset.concrete += n,
            Expr::Var(v) => self.offset.abs.push(v),
        }
        self
    }

    /// Attempt to add a [TimeSub] expression to this time expression.
    /// Only possible when the [TimeSub::Unit] expression is a concrete value.
    pub fn try_increment(self, sub: TimeSub) -> Result<Self, (Self, TimeSub)> {
        match sub {
            TimeSub::Unit(n) => Ok(self.increment(n)),
            TimeSub::Sym { .. } => Err((self, sub)),
        }
    }

    /// Resolve the events bound in this time expression
    pub fn resolve_event(&self, bindings: &utils::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.concrete += self.offset.concrete;
        n.offset.abs.extend(self.offset.abs.clone());
        n
    }

    /// Resolve all expressions bound in this time expression
    pub fn resolve_expr(&self, bind: &utils::Binding<Expr>) -> Self {
        let mut offset = TimeSum {
            concrete: self.offset.concrete,
            abs: vec![],
        };

        for x in &self.offset.abs {
            match bind.find(x) {
                Some(Expr::Const(c)) => offset.concrete += c,
                Some(Expr::Var(v)) => offset.abs.push(v.clone()),
                None => offset.abs.push(x.clone()),
            }
        }

        Time {
            event: self.event.clone(),
            offset,
        }
    }

    /// Get the event associated with this time expression
    pub fn event(&self) -> Id {
        self.event.clone()
    }
}

impl From<Id> for Time {
    fn from(event: Id) -> Self {
        Time::unit(event, 0)
    }
}

impl From<Time> for SExp {
    fn from(t: Time) -> SExp {
        if t.offset.abs.is_empty() && t.offset.concrete == 0 {
            SExp(format!("{}", t.event))
        } else if t.offset.abs.is_empty() {
            SExp(format!("(+ {} {})", t.event, t.offset.concrete))
        } else {
            SExp(format!(
                "(+ {} {} {})",
                t.event,
                t.offset.concrete,
                t.offset
                    .abs
                    .iter()
                    .map(|e| e.to_string())
                    .collect_vec()
                    .join(" ")
            ))
        }
    }
}

impl Range {
    /// Convert this interval into an offset. Only possible when interval uses
    /// exactly one event for both start and end.
    pub fn as_offset(&self) -> Option<(Id, TimeSum, TimeSum)> {
        let Range { start, end, .. } = &self;
        if start.event == end.event {
            Some((
                start.event.clone(),
                start.offset.clone(),
                end.offset().clone(),
            ))
        } else {
            None
        }
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.event)?;
        for x in &self.offset.abs {
            write!(f, "+{x}")?;
        }
        if self.offset.concrete != 0 {
            write!(f, "+{}", self.offset.concrete)?;
        }
        Ok(())
    }
}

impl std::ops::Sub for Time {
    type Output = TimeSub;

    fn sub(self, other: Self) -> Self::Output {
        let lc = self.offset().concrete;
        let rc = other.offset().concrete;
        if self.event == other.event && self.offset().abs == other.offset().abs
        {
            TimeSub::Unit(u64::abs_diff(lc, rc).into())
        } else {
            // Only add abstract variable when neither side has it.
            let mut abs = vec![];
            for a in &self.offset().abs {
                if !other.offset().abs.contains(a) {
                    abs.push(a.clone());
                }
            }

            // If the left side has more concrete time, then the right side
            // is the one that is subtracted.
            if lc > rc {
                TimeSub::Sym {
                    l: Time {
                        event: self.event,
                        offset: TimeSum {
                            concrete: lc - rc,
                            abs,
                        },
                    },
                    r: other.event.into(),
                }
            } else {
                TimeSub::Sym {
                    l: self.event.into(),
                    r: Time {
                        event: other.event,
                        offset: TimeSum {
                            concrete: rc - lc,
                            abs,
                        },
                    },
                }
            }
        }
    }
}

/// Represents the absolute difference between two time events
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TimeSub {
    /// Concrete difference between two time expressions
    Unit(Expr),
    /// Symbolic difference between two time expressions
    Sym { l: Time, r: Time },
}

impl TimeSub {
    pub fn unit(n: Expr) -> Self {
        TimeSub::Unit(n)
    }

    pub fn concrete(&self) -> Option<u64> {
        match self {
            TimeSub::Unit(Expr::Const(n)) => Some(*n),
            _ => None,
        }
    }
}

impl TimeSub {
    pub fn resolve_event(&self, bindings: &utils::Binding<Time>) -> Self {
        match self {
            // Unit cannot contain any events
            TimeSub::Unit(_) => self.clone(),
            TimeSub::Sym { l, r } => {
                l.resolve_event(bindings) - r.resolve_event(bindings)
            }
        }
    }

    pub fn resolve_offset(&self, bindings: &utils::Binding<Expr>) -> Self {
        match self {
            TimeSub::Unit(n) => TimeSub::Unit(n.resolve(bindings).unwrap()),
            TimeSub::Sym { l, r } => {
                l.resolve_expr(bindings) - r.resolve_expr(bindings)
            }
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

impl From<Expr> for TimeSub {
    fn from(n: Expr) -> Self {
        TimeSub::Unit(n)
    }
}
