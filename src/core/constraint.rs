use derivative::Derivative;

use super::{Expr, Id, Range, Time, TimeSub};
use crate::diagnostics::{self, InfoIdx};
use crate::errors::Error;
use crate::utils::Binding;
use crate::utils::SExp;
use std::fmt::Display;

/// Ordering operator for constraints
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum OrderOp {
    Gt,
    Gte,
    Eq,
}
impl std::fmt::Display for OrderOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self {
            OrderOp::Gt => ">",
            OrderOp::Eq => "=",
            OrderOp::Gte => ">=",
        };
        write!(f, "{op}")
    }
}

// An ordering constraint
#[derive(Clone, Derivative, Eq)]
#[derivative(PartialEq, Hash)]
pub struct OrderConstraint<T> {
    left: T,
    right: T,
    op: OrderOp,
    // Explanation of why this constraint was generated
    #[derivative(PartialEq = "ignore")]
    #[derivative(Hash = "ignore")]
    extra: Vec<diagnostics::InfoIdx>,
}

impl<T> OrderConstraint<T>
where
    T: Clone,
{
    pub fn new(left: T, right: T, op: OrderOp) -> Self {
        Self {
            left,
            right,
            op,
            extra: vec![],
        }
    }

    pub fn lt(l: T, r: T) -> Self {
        Self {
            left: r,
            right: l,
            op: OrderOp::Gt,
            extra: vec![],
        }
    }

    pub fn eq(left: T, right: T) -> Self {
        OrderConstraint {
            left,
            right,
            op: OrderOp::Eq,
            extra: vec![],
        }
    }

    pub fn gte(left: T, right: T) -> Self {
        OrderConstraint {
            left,
            right,
            op: OrderOp::Gte,
            extra: vec![],
        }
    }

    pub fn lte(l: T, r: T) -> Self {
        OrderConstraint {
            left: r,
            right: l,
            op: OrderOp::Gte,
            extra: vec![],
        }
    }
}

impl OrderConstraint<Time> {
    fn resolve_event(&self, bindings: &Binding<Time>) -> Self {
        OrderConstraint {
            left: self.left.resolve_event(bindings),
            right: self.right.resolve_event(bindings),
            ..self.clone()
        }
    }

    fn resolve_offset(&self, bindings: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve_offset(bindings),
            right: self.right.resolve_offset(bindings),
            ..self.clone()
        }
    }
}
impl OrderConstraint<TimeSub> {
    fn resolve_event(&self, bindings: &Binding<Time>) -> Self {
        OrderConstraint {
            left: self.left.resolve_event(bindings),
            right: self.right.resolve_event(bindings),
            ..self.clone()
        }
    }

    fn resolve_offset(&self, bindings: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve_offset(bindings),
            right: self.right.resolve_offset(bindings),
            ..self.clone()
        }
    }
}

impl OrderConstraint<Time> {
    /// Check that the `left` range is equal to `right`
    pub fn equality(left: Range, right: Range) -> impl Iterator<Item = Self> {
        log::trace!("{left} = {right}");
        vec![
            OrderConstraint::eq(left.start, right.start),
            OrderConstraint::eq(left.end, right.end),
        ]
        .into_iter()
    }

    /// Check that the `left` range is a subset of `right`
    /// [ls, le] \subsetof [rs, re] <=> rs <= ls <= le <= re
    pub fn subset(left: Range, right: Range) -> impl Iterator<Item = Self> {
        log::trace!("{left} ⊆ {right}");
        vec![
            OrderConstraint::lte(right.start, left.start),
            OrderConstraint::gte(right.end, left.end),
        ]
        .into_iter()
    }
}

impl<T> Display for OrderConstraint<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.left, self.op, self.right)?;
        Ok(())
    }
}

impl<T> From<OrderConstraint<T>> for SExp
where
    SExp: From<T>,
{
    fn from(c: OrderConstraint<T>) -> Self {
        SExp(format!(
            "({} {} {})",
            c.op,
            SExp::from(c.left),
            SExp::from(c.right)
        ))
    }
}

/// A ordering constraint over time expressions or time ranges.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Constraint {
    Base {
        base: OrderConstraint<Time>,
    },
    /// Represents ordering over time ranges.
    Sub {
        base: OrderConstraint<TimeSub>,
    },
}

impl Constraint {
    pub fn base(base: OrderConstraint<Time>) -> Self {
        Self::Base { base }
    }

    pub fn sub(base: OrderConstraint<TimeSub>) -> Self {
        Self::Sub { base }
    }

    /// Create a new constraint that `l` is less than `r`
    pub fn lt(l: Time, r: Time) -> Self {
        Self::Base {
            base: OrderConstraint::lt(l, r),
        }
    }

    pub fn notes(&self) -> impl Iterator<Item = &diagnostics::InfoIdx> {
        match self {
            Constraint::Base { base } => base.extra.iter(),
            Constraint::Sub { base } => base.extra.iter(),
        }
    }

    pub fn add_note(mut self, note: InfoIdx) -> Self {
        match &mut self {
            Constraint::Base { base } => base.extra.push(note),
            Constraint::Sub { base } => base.extra.push(note),
        }
        self
    }

    pub fn events(&self) -> Vec<Id> {
        match self {
            Constraint::Base { base } => {
                vec![base.left.event(), base.right.event()]
            }
            Constraint::Sub { base } => {
                let mut evs = base.left.events();
                evs.append(&mut base.right.events());
                evs
            }
        }
    }

    /// Check if this constraint is an ordering constraint
    pub fn is_ordering(&self) -> bool {
        match self {
            Constraint::Base { base } => base.op != OrderOp::Eq,
            Constraint::Sub { base } => base.op != OrderOp::Eq,
        }
    }
}

impl Constraint {
    pub fn resolve_event(&self, binding: &Binding<Time>) -> Constraint {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve_event(binding),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: base.resolve_event(binding),
            },
        }
    }

    pub fn resolve_offset(&self, bindings: &Binding<Expr>) -> Self {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve_offset(bindings),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: base.resolve_offset(bindings),
            },
        }
    }
}

impl Display for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::Base { base } => write!(f, "{}", base),
            Constraint::Sub { base } => write!(f, "{}", base),
        }
    }
}

impl From<Constraint> for SExp {
    fn from(con: Constraint) -> Self {
        match con {
            Constraint::Base { base } => SExp(format!(
                "({} {} {})",
                base.op,
                SExp::from(base.left),
                SExp::from(base.right),
            )),
            Constraint::Sub { base } => SExp(format!(
                "({} {} {})",
                base.op,
                SExp::from(base.left),
                SExp::from(base.right),
            )),
        }
    }
}

impl From<Constraint> for Error {
    fn from(con: Constraint) -> Self {
        let mut err = Error::malformed(format!("Cannot prove: {}", con));
        err.notes = con.notes().cloned().collect();
        err
    }
}
