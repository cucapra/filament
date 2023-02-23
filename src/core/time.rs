use super::{Expr, Id, Range};
use crate::utils::{self, SExp};
use std::fmt::Display;

#[derive(Clone, Hash)]
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
            offset: Expr::concrete(state),
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
    pub fn resolve_event(self, bindings: &utils::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset += self.offset;
        n
    }

    /// Resolve all expressions bound in this time expression
    pub fn resolve_expr(self, bind: &utils::Binding<Expr>) -> Self {
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
    fn from(value: Time) -> Self {
        SExp(format!("(+ {} {})", value.event, SExp::from(value.offset)))
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
        if self.event == other.event {
            return TimeSub::Unit(self.offset - other.offset);
        }

        TimeSub::Sym { l: self, r: other }
    }
}

/// Represents the absolute difference between two time events
#[derive(Clone, Hash)]
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
            TimeSub::Unit(n) => u64::try_from(n).ok(),
            _ => None,
        }
    }

    /// Resolve events bound in this time expression
    pub fn resolve_event(self, bindings: &utils::Binding<Time>) -> Self {
        match self {
            // Unit cannot contain any events
            TimeSub::Unit(_) => self.clone(),
            TimeSub::Sym { l, r } => {
                l.resolve_event(bindings) - r.resolve_event(bindings)
            }
        }
    }

    pub fn resolve_expr(self, bindings: &utils::Binding<Expr>) -> Self {
        match self {
            TimeSub::Unit(n) => TimeSub::Unit(n.resolve(bindings)),
            TimeSub::Sym { l, r } => {
                l.resolve_expr(bindings) - r.resolve_expr(bindings)
            }
        }
    }

    pub fn events(&self) -> Vec<&Time> {
        match self {
            TimeSub::Unit(_) => vec![],
            TimeSub::Sym { l, r } => {
                vec![l, r]
            }
        }
    }

    pub fn exprs(&self) -> Vec<&Expr> {
        match self {
            TimeSub::Unit(n) => vec![n],
            TimeSub::Sym { l, r } => {
                vec![&l.offset, &r.offset]
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
            TimeSub::Unit(n) => n.into(),
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
