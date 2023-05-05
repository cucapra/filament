use super::{Expr, Range, Time, TimeSub};
use crate::errors::FilamentResult;
use crate::utils::Binding;
use crate::utils::Obligation;
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
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct OrderConstraint<T> {
    pub left: T,
    pub right: T,
    pub op: OrderOp,
}

impl<T> OrderConstraint<T>
where
    T: Clone,
{
    pub fn new(left: T, right: T, op: OrderOp) -> Self {
        Self { left, right, op }
    }

    pub fn is_eq(&self) -> bool {
        self.op == OrderOp::Eq
    }

    pub fn gt(l: T, r: T) -> Self {
        Self {
            left: l,
            right: r,
            op: OrderOp::Gt,
        }
    }

    pub fn lt(l: T, r: T) -> Self {
        Self {
            left: r,
            right: l,
            op: OrderOp::Gt,
        }
    }

    pub fn eq(left: T, right: T) -> Self {
        OrderConstraint {
            left,
            right,
            op: OrderOp::Eq,
        }
    }

    pub fn gte(left: T, right: T) -> Self {
        OrderConstraint {
            left,
            right,
            op: OrderOp::Gte,
        }
    }

    pub fn lte(l: T, r: T) -> Self {
        OrderConstraint {
            left: r,
            right: l,
            op: OrderOp::Gte,
        }
    }
}

impl<T> OrderConstraint<T>
where
    SExp: From<OrderConstraint<T>>,
{
    pub fn obligation<S: ToString>(self, reason: S) -> Obligation {
        Obligation::new(SExp::from(self), reason.to_string())
    }
}

impl OrderConstraint<Expr> {
    pub fn resolve_expr(self, binding: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            ..self
        }
    }

    pub fn eval(self, binding: &Binding<Expr>) -> FilamentResult<bool> {
        let OrderConstraint { left, right, op } = self.resolve_expr(binding);
        let l: u64 = left.try_into()?;
        let r: u64 = right.try_into()?;
        Ok(match op {
            OrderOp::Gt => l > r,
            OrderOp::Gte => l >= r,
            OrderOp::Eq => l == r,
        })
    }
}

impl OrderConstraint<Time> {
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        OrderConstraint {
            left: self.left.resolve_event(bindings),
            right: self.right.resolve_event(bindings),
            ..self
        }
    }

    pub fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve_expr(bindings),
            right: self.right.resolve_expr(bindings),
            ..self
        }
    }
}

impl OrderConstraint<TimeSub> {
    fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        OrderConstraint {
            left: self.left.resolve_event(bindings),
            right: self.right.resolve_event(bindings),
            ..self
        }
    }

    fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        OrderConstraint {
            left: self.left.resolve_expr(bindings),
            right: self.right.resolve_expr(bindings),
            ..self
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
#[derive(Clone, Hash)]
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

    pub fn events(&self) -> Vec<&Time> {
        match self {
            Constraint::Base { base } => {
                vec![&base.left, &base.right]
            }
            Constraint::Sub { base } => {
                let mut evs = base.left.events();
                evs.append(&mut base.right.events());
                evs
            }
        }
    }

    /// All expressions used in this constraint
    pub fn exprs(&self) -> Vec<&Expr> {
        match self {
            Constraint::Base { base } => {
                vec![base.left.offset(), base.right.offset()]
            }
            Constraint::Sub { base } => {
                let mut evs = base.left.exprs();
                evs.append(&mut base.right.exprs());
                evs
            }
        }
    }

    pub fn resolve_event(self, binding: &Binding<Time>) -> Constraint {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve_event(binding),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: base.resolve_event(binding),
            },
        }
    }

    pub fn resolve_expr(self, bindings: &Binding<Expr>) -> Self {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve_expr(bindings),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: base.resolve_expr(bindings),
            },
        }
    }

    /// Generate an obligation for this constraint and provide a reason
    pub fn obligation<S: ToString>(self, reason: S) -> Obligation {
        Obligation::new(SExp::from(self), reason.to_string())
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

impl From<OrderConstraint<Time>> for Constraint {
    fn from(con: OrderConstraint<Time>) -> Self {
        Constraint::Base { base: con }
    }
}

impl From<OrderConstraint<TimeSub>> for Constraint {
    fn from(con: OrderConstraint<TimeSub>) -> Self {
        Constraint::Sub { base: con }
    }
}
