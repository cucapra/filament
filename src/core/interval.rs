use std::collections::HashMap;

use crate::interval_checking::SExp;

use super::Id;

/// Represents a time variable which can either be:
///   1. An abstract variable like `G`.
///   2. A concrete time such as 1.
///   3. A binary operation of two other interval times.
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum IntervalBase {
    Abstract(Id),
    Concrete(u64),
    Next { base: Id, incr: u64 },
}
impl std::fmt::Debug for IntervalBase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntervalBase::Abstract(name) => write!(f, "{}", name),
            IntervalBase::Concrete(n) => write!(f, "{}", n),
            IntervalBase::Next { base, incr } => write!(f, "{}+{}", base, incr),
        }
    }
}
impl From<&IntervalBase> for SExp {
    fn from(ib: &IntervalBase) -> Self {
        match ib {
            IntervalBase::Abstract(name) => SExp(format!("{}", name)),
            IntervalBase::Concrete(n) => SExp(format!("{}", n)),
            IntervalBase::Next { base, incr } => {
                SExp(format!("(+ {} {})", base, incr))
            }
        }
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub enum IntervalTime {
    Base(IntervalBase),
    Max {
        left: Box<IntervalTime>,
        right: Box<IntervalTime>,
    },
}
impl IntervalTime {
    /// Construct an [IntervalTime::Abstract].
    #[inline]
    pub fn abs(time_var: Id) -> Self {
        IntervalTime::Base(IntervalBase::Abstract(time_var))
    }

    pub fn binop_max(left: IntervalTime, right: IntervalTime) -> Self {
        match (left, right) {
            (
                IntervalTime::Base(IntervalBase::Concrete(n1)),
                IntervalTime::Base(IntervalBase::Concrete(n2)),
            ) => {
                if n1 > n2 {
                    Self::concrete(n1)
                } else {
                    Self::concrete(n2)
                }
            }
            (l, r) => IntervalTime::Max {
                left: Box::new(l),
                right: Box::new(r),
            },
        }
    }

    /// Construct a binop add instance and try to peephole optimize the construction
    pub fn binop_add(base: Id, incr: u64) -> Self {
        IntervalTime::Base(IntervalBase::Next { base, incr })
    }

    #[inline]
    pub fn concrete(num: u64) -> Self {
        IntervalTime::Base(IntervalBase::Concrete(num))
    }

    /// Resolve the IntervalTime using the given bindings from abstract variables to exact
    /// bindings.
    pub fn resolve(&self, bindings: &HashMap<Id, IntervalTime>) -> Self {
        match self {
            IntervalTime::Base(base) => match base {
                IntervalBase::Abstract(name) => bindings
                    .get(name)
                    .unwrap_or_else(|| panic!("No binding for {}", name))
                    .clone(),
                IntervalBase::Concrete(_) => IntervalTime::Base(base.clone()),
                IntervalBase::Next { base, incr } => {
                    let res = bindings
                        .get(base)
                        .unwrap_or_else(|| panic!("No binding for {}", base));
                    match res {
                        IntervalTime::Base(b) => match b {
                            IntervalBase::Abstract(new_base) => {
                                Self::binop_add(new_base.clone(), *incr)
                            }
                            IntervalBase::Concrete(n) => {
                                Self::concrete(n + incr)
                            }
                            IntervalBase::Next { base, incr: n } => {
                                Self::binop_add(base.clone(), n + incr)
                            }
                        },
                        IntervalTime::Max { .. } => {
                            todo!("Max appeared in the resolution")
                        }
                    }
                }
            },
            IntervalTime::Max { left, right } => IntervalTime::Max {
                left: Box::new(left.resolve(bindings)),
                right: Box::new(right.resolve(bindings)),
            },
        }
    }
}
impl std::fmt::Debug for IntervalTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IntervalTime::Base(base) => write!(f, "{:?}", base),
            IntervalTime::Max { left, right } => {
                write!(f, "max(")?;
                left.fmt(f)?;
                write!(f, ",")?;
                right.fmt(f)?;
                write!(f, ")")
            }
        }
    }
}

/// Ordering operator for constraints
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum OrderOp {
    Gt,
    Lt,
    Eq,
}

/// A constraint on time expressions
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct Constraint {
    pub left: IntervalTime,
    pub right: IntervalTime,
    pub op: OrderOp,
}
impl Constraint {
    pub fn resolve(&self, binding: &HashMap<Id, IntervalTime>) -> Constraint {
        Constraint {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            op: self.op.clone(),
        }
    }
}
impl std::fmt::Debug for Constraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} ", self.left)?;
        write!(
            f,
            "{}",
            match self.op {
                OrderOp::Gt => ">",
                OrderOp::Lt => "<",
                OrderOp::Eq => "=",
            }
        )?;
        write!(f, " {:?}", self.right)
    }
}
impl From<&Constraint> for SExp {
    fn from(con: &Constraint) -> Self {
        let op_str = match con.op {
            OrderOp::Gt => ">",
            OrderOp::Lt => "<",
            OrderOp::Eq => "=",
        };
        SExp(format!(
            "({} {} {})",
            op_str,
            SExp::from(&con.left),
            SExp::from(&con.right)
        ))
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
            IntervalTime::Base(base) => SExp::from(base),
            IntervalTime::Max { left, right } => SExp(format!(
                "(max {} {})",
                SExp::from(&**left),
                SExp::from(&**right)
            )),
        }
    }
}
