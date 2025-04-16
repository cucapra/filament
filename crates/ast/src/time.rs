use super::{Binding, Expr, Id};
use crate::Loc;
use std::fmt::Display;

#[derive(Clone, Hash)]
pub enum MaybeUnknown {
    /// A concrete value
    Known(Expr),
    /// An unknown variable binding used in scheduling.
    /// Represents the uninterpreted function f(args)
    Unknown(Vec<Loc<Id>>),
}

impl MaybeUnknown {
    pub fn as_known(&self) -> Option<&Expr> {
        match self {
            MaybeUnknown::Known(expr) => Some(expr),
            MaybeUnknown::Unknown(_) => None,
        }
    }

    pub fn known(&self) -> &Expr {
        self.as_known().unwrap_or_else(|| {
            panic!("Tried to get known value from unknown binding")
        })
    }

    pub fn is_known(&self) -> bool {
        matches!(self, MaybeUnknown::Known(_))
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, MaybeUnknown::Unknown(_))
    }
}
#[derive(Clone, Hash)]
/// Represents expression of the form `G+1+k`
pub struct Time {
    /// The event for the time expression
    pub event: Id,
    /// The offsets for the time expression
    /// May be `None` if we have a 'G + ? binding.
    pub offset: MaybeUnknown,
}

impl Time {
    pub fn new(event: Id, offset: MaybeUnknown) -> Self {
        Self { event, offset }
    }

    /// Get the offset associated with this time expression
    pub fn offset(&self) -> &Expr {
        self.offset.as_known().unwrap()
    }

    /// Unit time expression that occurs when the event occurs
    pub fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: MaybeUnknown::Known(Expr::concrete(state)),
        }
    }

    /// Resolve the events bound in this time expression
    pub fn resolve_event(self, bindings: &Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();

        // 'G + ? bindings should never be attempted to be resolved
        let (MaybeUnknown::Known(so), MaybeUnknown::Known(no)) =
            (self.offset, n.offset)
        else {
            unreachable!(
                "Time expressions with ? should never be used when resolving"
            )
        };

        n.offset = MaybeUnknown::Known(so + no);

        n
    }

    /// Resolve all expressions bound in this time expression
    pub fn resolve_expr(self, bind: &Binding<Expr>) -> Self {
        let MaybeUnknown::Known(offset) = self.offset else {
            unreachable!(
                "Time expressions with ? should never be used when resolving"
            )
        };

        Time {
            event: self.event,
            offset: MaybeUnknown::Known(offset.resolve(bind)),
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
                .as_known()
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
            let (MaybeUnknown::Known(o1), MaybeUnknown::Known(o2)) =
                (self.offset, other.offset)
            else {
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
