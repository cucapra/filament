use super::{FsmIdxs, Id, Range, TimeRep};
use crate::{
    errors::{self, FilamentResult},
    interval_checking::SExp,
};
use std::{cmp::Ordering, collections::HashMap, fmt::Display};

/// Ordering operator for constraints
#[derive(Hash, Eq, PartialEq, Clone)]
enum OrderOp {
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

type Extra = Vec<(String, Option<errors::Span>)>;

// A ordering constraint on time expressions
#[derive(Clone)]
pub struct ConstraintBase<T> {
    left: T,
    right: T,
    op: OrderOp,
    // Explanation of why this constraint was generated
    extra: Extra,
}
impl<T> ConstraintBase<T> {
    pub fn map<K, F: Fn(T) -> FilamentResult<K>>(
        self,
        f: F,
    ) -> FilamentResult<ConstraintBase<K>> {
        Ok(ConstraintBase {
            left: f(self.left)?,
            right: f(self.right)?,
            op: self.op,
            extra: self.extra,
        })
    }
}

impl<T> ConstraintBase<T>
where
    T: Clone,
{
    pub fn lt(l: T, r: T) -> Self {
        Self {
            left: r,
            right: l,
            op: OrderOp::Gt,
            extra: vec![],
        }
    }

    pub fn eq(left: T, right: T) -> Self {
        ConstraintBase {
            left,
            right,
            op: OrderOp::Eq,
            extra: vec![],
        }
    }

    pub fn gte(left: T, right: T) -> Self {
        ConstraintBase {
            left,
            right,
            op: OrderOp::Gte,
            extra: vec![],
        }
    }
}

impl<T: TimeRep> ConstraintBase<T> {
    pub fn resolve(&self, binding: &HashMap<Id, &T>) -> ConstraintBase<T> {
        ConstraintBase {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            ..self.clone()
        }
    }

    /// Check that the `left` range is equal to `right`
    pub fn equality(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        log::info!("{left} = {right}");
        vec![
            ConstraintBase::eq(left.start, right.start),
            ConstraintBase::eq(left.end, right.end),
        ]
        .into_iter()
    }

    /// Check that the `left` range is a subset of `right`
    /// [ls, le] \subsetof [rs, re] <=> ls >= rs && le <= re
    pub fn subset(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        log::info!("{left} ⊆ {right}");
        vec![
            ConstraintBase::gte(left.start, right.start),
            ConstraintBase::gte(right.end, left.end),
        ]
        .into_iter()
    }
}

impl<T> Display for ConstraintBase<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.left, self.op, self.right)?;
        Ok(())
    }
}

#[derive(Clone)]
pub enum Constraint<T: TimeRep> {
    Base { base: ConstraintBase<T> },
    Sub { base: ConstraintBase<(T, T)> },
}

impl<T: TimeRep> From<ConstraintBase<(T, T)>> for Constraint<T> {
    fn from(base: ConstraintBase<(T, T)>) -> Self {
        Self::Sub { base }
    }
}

impl<T: TimeRep> From<ConstraintBase<T>> for Constraint<T> {
    fn from(base: ConstraintBase<T>) -> Self {
        Self::Base { base }
    }
}

impl<T: TimeRep> Constraint<T> {
    pub fn lt(l: T, r: T) -> Self {
        Self::Base {
            base: ConstraintBase {
                left: r,
                right: l,
                op: OrderOp::Gt,
                extra: vec![],
            },
        }
    }

    pub fn gte(l: T, r: T) -> Self {
        Self::Base {
            base: ConstraintBase {
                left: l,
                right: r,
                op: OrderOp::Gte,
                extra: vec![],
            },
        }
    }

    pub fn eq(l: T, r: T) -> Self {
        Self::Base {
            base: ConstraintBase {
                left: l,
                right: r,
                op: OrderOp::Eq,
                extra: vec![],
            },
        }
    }

    pub fn notes(
        &self,
    ) -> impl Iterator<Item = &(String, Option<errors::Span>)> {
        match self {
            Constraint::Base { base } => base.extra.iter(),
            Constraint::Sub { base } => base.extra.iter(),
        }
    }

    pub fn add_note<S: ToString>(
        mut self,
        msg: S,
        pos: Option<errors::Span>,
    ) -> Self {
        match &mut self {
            Constraint::Base { base } => {
                base.extra.push((msg.to_string(), pos))
            }
            Constraint::Sub { base } => base.extra.push((msg.to_string(), pos)),
        }
        self
    }

    pub fn map<K: TimeRep, F: Fn(T) -> FilamentResult<K>>(
        self,
        f: F,
    ) -> FilamentResult<Constraint<K>> {
        match self {
            Constraint::Base { base } => {
                Ok(Constraint::Base { base: base.map(f)? })
            }
            Constraint::Sub { base } => Ok(Constraint::Sub {
                base: base
                    .map(|(l, r)| f(l).and_then(|l| f(r).map(|r| (l, r))))?,
            }),
        }
    }

    pub fn resolve(&self, binding: &HashMap<Id, &T>) -> Constraint<T> {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve(binding),
            },
            Constraint::Sub { base } => {
                let (la, ra) = &base.left;
                let (lb, rb) = &base.right;
                let base = ConstraintBase {
                    left: (la.resolve(binding), ra.resolve(binding)),
                    right: (lb.resolve(binding), rb.resolve(binding)),
                    ..base.clone()
                };
                Constraint::Sub { base }
            }
        }
    }
}

impl<T> Constraint<T>
where
    T: TimeRep + PartialEq + PartialOrd,
{
    /// Check if the constraint can be statically reduced to true.
    pub fn simplify(self) -> Option<Self> {
        match &self {
            Constraint::Base { base } => {
                let ord = base.left.partial_cmp(&base.right);
                match (&base.op, ord) {
                    (
                        OrderOp::Gte,
                        Some(Ordering::Greater | Ordering::Equal),
                    )
                    | (OrderOp::Eq, Some(Ordering::Equal))
                    | (OrderOp::Gt, Some(Ordering::Greater)) => None,
                    _ => Some(self),
                }
            }
            Constraint::Sub { .. } => Some(self),
        }
    }
}

impl<T: Display + TimeRep> Display for Constraint<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::Base { base } => write!(f, "{}", base),
            Constraint::Sub { base } => {
                let (la, ra) = &base.left;
                let (lb, rb) = &base.right;
                write!(f, "{la}-{ra} {} {lb}-{rb}", base.op)
            }
        }
    }
}

impl From<&Constraint<FsmIdxs>> for SExp {
    fn from(con: &Constraint<FsmIdxs>) -> Self {
        match con {
            Constraint::Base { base } => SExp(format!(
                "({} {} {})",
                base.op,
                SExp::from(&base.left),
                SExp::from(&base.right)
            )),
            Constraint::Sub { base } => {
                let (la, ra) = &base.left;
                let (lb, rb) = &base.right;
                SExp(format!(
                    "((- {} {}) {} (- {} {}))",
                    base.op,
                    SExp::from(la),
                    SExp::from(ra),
                    SExp::from(lb),
                    SExp::from(rb),
                ))
            }
        }
    }
}
