use super::{Binding, Expr, Id};
use std::fmt::Display;

#[derive(Clone, Hash)]
/// Represents expression of the form `G+1+k`
pub struct Time {
    /// The event for the time expression
    pub event: Id,
    /// The offsets for the time expression
    /// May be `None` if we have a 'G + ? binding.
    pub offset: Option<Expr>,
}

impl Time {
    pub fn new(event: Id, offset: Option<Expr>) -> Self {
        Self { event, offset }
    }

    /// Get the offset associated with this time expression
    pub fn offset(&self) -> &Expr {
        self.offset.as_ref().unwrap()
    }

    /// Unit time expression that occurs when the event occurs
    pub fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: Some(Expr::concrete(state)),
        }
    }

    /// Resolve the events bound in this time expression
    pub fn resolve_event(self, bindings: &Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();

        // 'G + ? bindings should never be attempted to be resolved
        let (Some(so), Some(no)) = (self.offset, n.offset) else {
            unreachable!(
                "Time expressions with ? should never be used when resolving"
            )
        };

        n.offset = Some(so + no);

        n
    }

    /// Resolve all expressions bound in this time expression
    pub fn resolve_expr(self, bind: &Binding<Expr>) -> Self {
        let Some(offset) = self.offset else {
            unreachable!(
                "Time expressions with ? should never be used when resolving"
            )
        };

        Time {
            event: self.event,
            offset: Some(offset.resolve(bind)),
        }
    }

    /// Get the event associated with this time expression
    pub fn event(&self) -> Id {
        self.event
    }
}

impl From<Id> for Time {
    fn from(event: Id) -> Self {
        Time::unit(event, 0)
    }
}

impl Display for Time {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}+{}",
            self.event,
            self.offset
                .as_ref()
                .map(|o| format!("{}", o))
                .unwrap_or("?".to_string())
        )?;
        Ok(())
    }
}

impl std::ops::Sub for Time {
    type Output = TimeSub;

    fn sub(self, other: Self) -> Self::Output {
        if self.event == other.event {
            let (Some(o1), Some(o2)) = (self.offset, other.offset) else {
                unreachable!("Time expressions with ? cannot be operated on.")
            };

            TimeSub::Unit(o1 - o2)
        } else {
            TimeSub::Sym { l: self, r: other }
        }
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

    /// Resolve events bound in this time expression
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        match self {
            // Unit cannot contain any events
            TimeSub::Unit(_) => self.clone(),
            TimeSub::Sym { l, r } => {
                l.resolve_event(bindings) - r.resolve_event(bindings)
            }
        }
    }

    pub fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        match self {
            TimeSub::Unit(n) => TimeSub::Unit(n.resolve(bindings)),
            TimeSub::Sym { l, r } => {
                l.resolve_expr(bindings) - r.resolve_expr(bindings)
            }
        }
    }
}

impl From<Expr> for TimeSub {
    fn from(n: Expr) -> Self {
        TimeSub::Unit(n)
    }
}
