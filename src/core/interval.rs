use std::collections::HashMap;

use super::Id;

/// Possible operations over time variables.
#[derive(Clone)]
pub enum TimeOp {
    Add,
    Sub,
}

/// Represents a time variable which can either be:
///   1. An abstract variable like `G`.
///   2. A concrete time such as 1.
///   3. A binary operation of two other interval times.
#[derive(Clone)]
pub enum IntervalTime {
    Abstract(Id),
    Concrete(u64),
    Port {
        cell: Id,
        name: Id,
    },
    BinOp {
        op: TimeOp,
        left: Box<IntervalTime>,
        right: Box<IntervalTime>,
    },
}
impl IntervalTime {
    /// Construct an [IntervalTime::Port].
    #[inline]
    pub fn port(cell: Id, name: Id) -> Self {
        IntervalTime::Port { cell, name }
    }

    /// Resolve the IntervalTime using the given bindings from abstract variables to exact
    /// bindings.
    pub fn resolve(&self, bindings: &HashMap<Id, IntervalTime>) -> Self {
        match self {
            IntervalTime::Concrete(_) | IntervalTime::Port { .. } => self.clone(),
            IntervalTime::Abstract(name) => bindings[name].clone(),
            IntervalTime::BinOp { op, left, right } => {
                IntervalTime::BinOp {
                    op: op.clone(),
                    left: Box::new(left.resolve(bindings)),
                    right: Box::new(right.resolve(bindings)),
                }
            }
        }
    }
}
impl std::fmt::Debug for IntervalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntervalTime::Abstract(id) => write!(f, "{}", id),
            IntervalTime::Concrete(n) => write!(f, "{}", n),
            IntervalTime::Port { cell, name } => write!(f, "{}.{}", cell, name),
            IntervalTime::BinOp { op, left, right } => {
                left.fmt(f)?;
                match op {
                    TimeOp::Add => write!(f, "+"),
                    TimeOp::Sub => write!(f, "-"),
                }?;
                right.fmt(f)
            }
        }
    }
}

/// Type of the interval which can either be:
///   1. Exact, implying set equality
///   2. Within, implying set containment.
#[derive(Clone)]
pub enum IntervalType {
    Exact,
    Within,
}

/// An interval consists of a type tag, a start time, and a end time.
pub struct Interval {
    pub tag: IntervalType,
    pub start: IntervalTime,
    pub end: IntervalTime,
}
impl Interval {
    pub fn resolve(&self, bindings: &HashMap<Id, IntervalTime>) -> Self {
        Interval {
            tag: self.tag.clone(),
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
        }
    }
}
impl std::fmt::Debug for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.tag {
            IntervalType::Exact => write!(f, "@exact")?,
            IntervalType::Within => write!(f, "@within")?,
        }
        write!(f, "(")?;
        self.start.fmt(f)?;
        write!(f, ", ")?;
        self.end.fmt(f)?;
        write!(f, ")")
    }
}
