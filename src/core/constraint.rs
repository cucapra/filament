use derivative::Derivative;

use super::{Binding, Id, Range, Time, TimeRep, TimeSub, WithTime};
use crate::{errors::FilamentResult, interval_checking::SExp, utils::GPosIdx};
use std::fmt::Display;

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

type Extra = Vec<(String, GPosIdx)>;

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
    extra: Extra,
}
impl<T> OrderConstraint<T> {
    pub fn map<K, F: Fn(T) -> FilamentResult<K>>(
        self,
        f: F,
    ) -> FilamentResult<OrderConstraint<K>> {
        Ok(OrderConstraint {
            left: f(self.left)?,
            right: f(self.right)?,
            op: self.op,
            extra: self.extra,
        })
    }
}

impl<T> OrderConstraint<T>
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
}

impl<K: TimeRep, T: WithTime<K>> WithTime<K> for OrderConstraint<T>
where
    Self: Clone,
{
    fn resolve(&self, bindings: &Binding<K>) -> Self {
        OrderConstraint {
            left: self.left.resolve(bindings),
            right: self.right.resolve(bindings),
            ..self.clone()
        }
    }
}

impl<T: TimeRep> OrderConstraint<T> {
    /// Check that the `left` range is equal to `right`
    pub fn equality(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        log::trace!("{left} = {right}");
        vec![
            OrderConstraint::eq(left.start, right.start),
            OrderConstraint::eq(left.end, right.end),
        ]
        .into_iter()
    }

    /// Check that the `left` range is a subset of `right`
    /// [ls, le] \subsetof [rs, re] <=> ls >= rs && le <= re
    pub fn subset(
        left: Range<T>,
        right: Range<T>,
    ) -> impl Iterator<Item = Self> {
        log::trace!("{left} ⊆ {right}");
        vec![
            OrderConstraint::gte(left.start, right.start),
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

/// A ordering constraint over time expressions or time ranges.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Constraint<T: TimeRep> {
    Base {
        base: OrderConstraint<T>,
    },
    /// Represents ordering over time ranges.
    Sub {
        base: OrderConstraint<TimeSub<T>>,
    },
}

impl<T: TimeRep> From<OrderConstraint<TimeSub<T>>> for Constraint<T> {
    fn from(base: OrderConstraint<TimeSub<T>>) -> Self {
        Self::Sub { base }
    }
}

impl<T: TimeRep> From<OrderConstraint<T>> for Constraint<T> {
    fn from(base: OrderConstraint<T>) -> Self {
        Self::Base { base }
    }
}

impl<T: TimeRep> Constraint<T> {
    /// Create a new constraint that `l` is less than `r`
    pub fn lt(l: T, r: T) -> Self {
        Self::Base {
            base: OrderConstraint::lt(l, r),
        }
    }

    pub fn notes(&self) -> impl Iterator<Item = &(String, GPosIdx)> {
        match self {
            Constraint::Base { base } => base.extra.iter(),
            Constraint::Sub { base } => base.extra.iter(),
        }
    }

    pub fn add_note<S: ToString>(mut self, msg: S, pos: GPosIdx) -> Self {
        match &mut self {
            Constraint::Base { base } => {
                base.extra.push((msg.to_string(), pos))
            }
            Constraint::Sub { base } => base.extra.push((msg.to_string(), pos)),
        }
        self
    }

    pub fn map<K, F>(self, f: F) -> FilamentResult<Constraint<K>>
    where
        K: TimeRep,
        F: Fn(T) -> FilamentResult<K>,
    {
        match self {
            Constraint::Base { base } => {
                Ok(Constraint::Base { base: base.map(f)? })
            }
            Constraint::Sub { .. } => todo!("Mapping over Constraint::Sub"),
        }
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

impl<T: TimeRep> WithTime<T> for Constraint<T> {
    fn resolve(&self, binding: &Binding<T>) -> Constraint<T> {
        match self {
            Constraint::Base { base } => Constraint::Base {
                base: base.resolve(binding),
            },
            Constraint::Sub { base } => Constraint::Sub {
                base: OrderConstraint {
                    left: base.left.resolve(binding),
                    right: base.right.resolve(binding),
                    ..base.clone()
                },
            },
        }
    }
}

impl<T: Display + TimeRep> Display for Constraint<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Constraint::Base { base } => write!(f, "{}", base),
            Constraint::Sub { base } => write!(f, "{}", base),
        }
    }
}

impl From<&Constraint<Time<u64>>> for SExp {
    fn from(con: &Constraint<Time<u64>>) -> Self {
        match con {
            Constraint::Base { base } => SExp(format!(
                "({} {} {})",
                base.op,
                SExp::from(&base.left),
                SExp::from(&base.right)
            )),
            Constraint::Sub { base } => SExp(format!(
                "({} {} {})",
                base.op,
                SExp::from(&base.left),
                SExp::from(&base.right),
            )),
        }
    }
}
