use itertools::Itertools;

use super::{Expr, Id, Range, TimeSub};
use crate::utils::SExp;
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash)]
/// An opaque time sum expression
pub struct TimeSum {
    concrete: u64,
    abs: Vec<Id>,
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

#[derive(Clone, PartialEq, Eq, Hash)]
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

// impl From<ConcTime> for SExp {
//     fn from(value: ConcTime) -> Self {
//         SExp(format!(
//             "(+ {} {} {})",
//             value.event,
//             value.offset.concrete,
//             value
//                 .offset
//                 .abs
//                 .iter()
//                 .map(|x| x.to_string())
//                 .collect::<Vec<_>>()
//                 .join(" ")
//         ))
//     }
// }

impl Time {
    pub fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: vec![Expr::Const(state)].into(),
        }
    }

    pub fn increment(mut self, n: Expr) -> Self {
        match n {
            Expr::Const(n) => self.offset.concrete += n,
            Expr::Var(v) => self.offset.abs.push(v),
        }
        self
    }

    pub fn resolve_event(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.concrete += self.offset.concrete;
        n.offset.abs.extend(self.offset.abs.clone());
        n
    }

    pub fn resolve_offset(&self, bind: &super::Binding<Expr>) -> Self {
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

    pub fn event(&self) -> Id {
        self.event.clone()
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
                    r: Time::unit(other.event, 0),
                }
            } else {
                TimeSub::Sym {
                    l: Time::unit(self.event, 0),
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
