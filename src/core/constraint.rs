use std::{collections::HashMap, iter};

use crate::interval_checking::SExp;

use super::{FsmIdxs, Id, Range, TimeRep};

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

/// A ordering constraint on time expressions
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct Constraint<T>
where
    T: TimeRep,
{
    pub left: T,
    pub right: T,
    pub op: OrderOp,
}

impl<T> Constraint<T>
where
    T: TimeRep + Clone + PartialOrd,
{
    pub fn gt(l: T, r: T) -> Option<Self> {
        if l > r {
            None
        } else {
            Some(Self {
                left: l,
                right: r,
                op: OrderOp::Gt,
            })
        }
    }

    pub fn lt(l: T, r: T) -> Option<Self> {
        if l < r {
            None
        } else {
            Some(Self {
                left: r,
                right: l,
                op: OrderOp::Gt,
            })
        }
    }
}

impl<T> Constraint<T>
where
    T: TimeRep + Clone + PartialEq + PartialOrd,
{
    /// Check if the constraint can be statically reduced to true.
    pub fn simplify(&self) -> Option<&Self> {
        match self.op {
            OrderOp::Gt => {
                if self.left > self.right {
                    None
                } else {
                    Some(self)
                }
            }
            OrderOp::Gte => {
                if self.left >= self.right {
                    None
                } else {
                    Some(self)
                }
            }
            OrderOp::Eq => {
                if self.left == self.right {
                    None
                } else {
                    Some(self)
                }
            }
        }
    }

    pub fn eq(left: T, right: T) -> Self {
        Constraint {
            left,
            right,
            op: OrderOp::Eq,
        }
    }

    pub fn lte(left: T, right: T) -> Self {
        Constraint {
            left: right,
            right: left,
            op: OrderOp::Gte,
        }
    }

    pub fn gte(left: T, right: T) -> Self {
        Constraint {
            left,
            right,
            op: OrderOp::Gte,
        }
    }

    pub fn resolve(&self, binding: &HashMap<Id, &T>) -> Constraint<T> {
        Constraint {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            op: self.op.clone(),
        }
    }

    pub fn constraint(cons: Self) -> impl Iterator<Item = Self> {
        iter::once(cons)
    }

    /// Construct a [IntervalFact] with `tag` set to [FactType::Equality].
    pub fn equality(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        vec![
            Constraint::eq(left.start, right.start),
            Constraint::eq(left.end, right.end),
        ]
        .into_iter()
    }

    /// Construct a [IntervalFact] with `tag` set to [FactType::Subset].
    /// [ls, le] \subsetof [rs, re] <=> ls >= rs && le <= re
    pub fn subset(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        vec![
            Constraint::gte(left.start, right.start),
            Constraint::lte(left.end, right.end),
        ]
        .into_iter()
    }
}

impl<T> std::fmt::Debug for Constraint<T>
where
    T: std::fmt::Debug + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {} {:?}", self.left, self.op, self.right)
    }
}

impl From<&Constraint<FsmIdxs>> for SExp {
    fn from(con: &Constraint<FsmIdxs>) -> Self {
        SExp(format!(
            "({} {} {})",
            con.op,
            SExp::from(&con.left),
            SExp::from(&con.right)
        ))
    }
}
