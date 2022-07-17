use std::{cmp::Ordering, collections::HashMap, fmt::Display, iter};

use crate::{errors, interval_checking::SExp};

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
    // Explanation of why this constraint was generated
    extra: Vec<(String, Option<errors::Span>)>,
}

impl<T> Constraint<T>
where
    T: TimeRep,
{
    pub fn new(left: T, right: T, op: OrderOp) -> Self {
        Self {
            left,
            right,
            op,
            extra: vec![],
        }
    }

    pub fn notes(
        &self,
    ) -> impl Iterator<Item = &(String, Option<errors::Span>)> {
        self.extra.iter()
    }

    pub fn add_note<S: ToString>(
        mut self,
        msg: S,
        pos: Option<errors::Span>,
    ) -> Self {
        self.extra.push((msg.to_string(), pos));
        self
    }
}

impl<T> Constraint<T>
where
    T: TimeRep + Clone + PartialOrd,
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
        Constraint {
            left,
            right,
            op: OrderOp::Eq,
            extra: vec![],
        }
    }

    pub fn gte(left: T, right: T) -> Self {
        Constraint {
            left,
            right,
            op: OrderOp::Gte,
            extra: vec![],
        }
    }
    pub fn resolve(&self, binding: &HashMap<Id, &T>) -> Constraint<T> {
        Constraint {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            ..self.clone()
        }
    }

    pub fn constraint(cons: Self) -> impl Iterator<Item = Self> {
        iter::once(cons)
    }

    /// Check that the `left` range is equal to `right`
    pub fn equality(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        log::info!("{left} = {right}");
        vec![
            Constraint::eq(left.start, right.start),
            Constraint::eq(left.end, right.end),
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
            Constraint::gte(left.start, right.start),
            Constraint::gte(right.end, left.end),
        ]
        .into_iter()
    }
}

impl<T> Constraint<T>
where
    T: TimeRep + Clone + PartialEq + PartialOrd,
{
    /// Check if the constraint can be statically reduced to true.
    pub fn simplify(self) -> Option<Self> {
        let ord = self.left.partial_cmp(&self.right);
        match (&self.op, ord) {
            (OrderOp::Gte, Some(Ordering::Greater | Ordering::Equal))
            | (OrderOp::Eq, Some(Ordering::Equal))
            | (OrderOp::Gt, Some(Ordering::Greater)) => None,
            _ => Some(self),
        }
    }
}

impl<T> Display for Constraint<T>
where
    T: Display + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.left, self.op, self.right)?;
        Ok(())
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
