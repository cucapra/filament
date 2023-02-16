use super::{Expr, Id, Range};
use crate::utils::{self, SExp};
use itertools::Itertools;
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd)]
/// Represents expression of the form `G+1+k`
pub struct Time {
    /// The event for the time expression
    pub event: Id,
    /// The offsets for the time expression
    offset: Expr,
}

impl Time {
    pub fn new(event: Id, offset: Expr) -> Self {
        Self { event, offset }
    }

    /// Get the offset associated with this time expression
    pub fn offset(&self) -> &Expr {
        &self.offset
    }

    /// Unit time expression that occurs when the event occurs
    pub fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: Expr::new(state, vec![]),
        }
    }

    /// Time expression that occurs `n` cycles after this time expression
    pub fn increment(mut self, n: Expr) -> Self {
        self.offset += n;
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
        Time {
            event: self.event.clone(),
            offset: self.offset.resolve(bind),
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
    pub fn as_offset(&self) -> Option<(Id, Expr, Expr)> {
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
        if self.offset.is_zero() {
            write!(f, "{}", self.event)?;
        } else {
            write!(f, "{}+{}", self.event, self.offset)?;
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
                        offset: Expr {
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
                        offset: Expr {
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

    /// Convert this time expression into a concrete value
    pub fn concrete(&self) -> Option<u64> {
        match self {
            TimeSub::Unit(n) => n.concrete(),
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
            TimeSub::Unit(n) => TimeSub::Unit(n.resolve(bindings)),
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
