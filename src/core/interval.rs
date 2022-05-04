use std::collections::HashMap;

use crate::interval_checking::SExp;

use super::Id;

/// Possible operations over time variables.
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum TimeOp {
    Add,
    Max,
}

/// Represents a time variable which can either be:
///   1. An abstract variable like `G`.
///   2. A concrete time such as 1.
///   3. A binary operation of two other interval times.
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum IntervalTime {
    Abstract(Id),
    Concrete(u64),
    BinOp {
        op: TimeOp,
        left: Box<IntervalTime>,
        right: Box<IntervalTime>,
    },
}
impl IntervalTime {
    /// Construct an [IntervalTime::Abstract].
    #[inline]
    pub fn abs(time_var: Id) -> Self {
        IntervalTime::Abstract(time_var)
    }

    pub fn binop_max(left: IntervalTime, right: IntervalTime) -> Self {
        match (left, right) {
            (IntervalTime::Concrete(n1), IntervalTime::Concrete(n2)) => {
                if n1 > n2 {
                    Self::concrete(n1)
                } else {
                    Self::concrete(n2)
                }
            }
            (l, r) => IntervalTime::BinOp {
                op: TimeOp::Max,
                left: Box::new(l),
                right: Box::new(r),
            },
        }
    }

    /// Construct a binop add instance and try to peephole optimize the construction
    pub fn binop_add(left: IntervalTime, right: IntervalTime) -> Self {
        match (left, right) {
            (IntervalTime::Concrete(n1), IntervalTime::Concrete(n2)) => {
                IntervalTime::Concrete(n1 + n2)
            }
            (
                IntervalTime::Concrete(n1),
                IntervalTime::BinOp {
                    op: TimeOp::Add,
                    left: l,
                    right: r,
                },
            )
            | (
                IntervalTime::BinOp {
                    op: TimeOp::Add,
                    left: l,
                    right: r,
                },
                IntervalTime::Concrete(n1),
            ) => {
                if let IntervalTime::Concrete(n2) = &*l {
                    return IntervalTime::binop_add(
                        IntervalTime::Concrete(n1 + n2),
                        *r,
                    );
                }
                if let IntervalTime::Concrete(n2) = &*r {
                    return IntervalTime::binop_add(
                        IntervalTime::Concrete(n1 + n2),
                        *l,
                    );
                }
                let con = IntervalTime::Concrete(n1);
                let bin = IntervalTime::BinOp {
                    op: TimeOp::Add,
                    left: l,
                    right: r,
                };
                IntervalTime::BinOp {
                    op: TimeOp::Add,
                    left: Box::new(con),
                    right: Box::new(bin),
                }
            }
            (l, r) => IntervalTime::BinOp {
                op: TimeOp::Add,
                left: Box::new(l),
                right: Box::new(r),
            },
        }
    }

    #[inline]
    pub fn concrete(num: u64) -> Self {
        IntervalTime::Concrete(num)
    }

    /// Resolve the IntervalTime using the given bindings from abstract variables to exact
    /// bindings.
    pub fn resolve(&self, bindings: &HashMap<Id, IntervalTime>) -> Self {
        match self {
            IntervalTime::Concrete(_) => self.clone(),
            IntervalTime::Abstract(name) => bindings[name].clone(),
            IntervalTime::BinOp { op, left, right } => match op {
                TimeOp::Add => IntervalTime::binop_add(
                    left.resolve(bindings),
                    right.resolve(bindings),
                ),
                TimeOp::Max => IntervalTime::binop_max(
                    left.resolve(bindings),
                    right.resolve(bindings),
                ),
            },
        }
    }
}
impl std::fmt::Debug for IntervalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntervalTime::Abstract(id) => write!(f, "{}", id),
            IntervalTime::Concrete(n) => write!(f, "{}", n),
            IntervalTime::BinOp { op, left, right } => match op {
                TimeOp::Add => {
                    left.fmt(f)?;
                    write!(f, "+")?;
                    right.fmt(f)
                }
                TimeOp::Max => {
                    write!(f, "max(")?;
                    left.fmt(f)?;
                    write!(f, ",")?;
                    right.fmt(f)?;
                    write!(f, ")")
                }
            },
        }
    }
}

/// An interval consists of a type tag, a start time, and a end time.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Interval {
    pub start: IntervalTime,
    pub end: IntervalTime,
}
impl Interval {
    /// Construct a [Interval] with `tag` set to [IntervalTime::Exact].
    pub fn exact(start: IntervalTime, end: IntervalTime) -> Self {
        Interval { start, end }
    }

    pub fn resolve(&self, bindings: &HashMap<Id, IntervalTime>) -> Self {
        Interval {
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
        }
    }
}
impl std::fmt::Debug for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@[")?;
        self.start.fmt(f)?;
        write!(f, ", ")?;
        self.end.fmt(f)?;
        write!(f, "]")
    }
}
impl From<&IntervalTime> for SExp {
    fn from(it: &IntervalTime) -> Self {
        match it {
            IntervalTime::Abstract(x) => SExp(x.to_string()),
            IntervalTime::Concrete(x) => SExp(x.to_string()),
            IntervalTime::BinOp { op, left, right } => match op {
                TimeOp::Add => SExp(format!(
                    "(+ {} {})",
                    SExp::from(&**left),
                    SExp::from(&**right),
                )),
                TimeOp::Max => todo!("Converting max operations to SExp"),
            },
        }
    }
}
