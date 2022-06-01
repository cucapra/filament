use std::collections::HashMap;

use crate::interval_checking::SExp;

use super::{FsmIdxs, Id, IntervalTime, TimeRep};

/// Ordering operator for constraints
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum OrderOp {
    Gt,
    Lt,
    Eq,
}

/// A constraint on time expressions
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
    T: TimeRep,
{
    pub fn resolve(&self, binding: &HashMap<Id, T>) -> Constraint<T> {
        Constraint {
            left: self.left.resolve(binding),
            right: self.right.resolve(binding),
            op: self.op.clone(),
        }
    }
}
impl<T> std::fmt::Debug for Constraint<T>
where
    T: std::fmt::Debug + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self.op {
            OrderOp::Gt => ">",
            OrderOp::Lt => "<",
            OrderOp::Eq => "=",
        };
        write!(f, "{:?} {op} {:?}", self.left, self.right)
    }
}

impl From<&Constraint<FsmIdxs>> for SExp {
    fn from(con: &Constraint<FsmIdxs>) -> Self {
        let op_str = match con.op {
            OrderOp::Gt => ">",
            OrderOp::Lt => "<",
            OrderOp::Eq => "=",
        };
        SExp(format!(
            "({op_str} {} {})",
            SExp::from(&con.left),
            SExp::from(&con.right)
        ))
    }
}

/// Tag describing the kind of interval constraint
#[derive(Clone, Hash, PartialEq, Eq)]
pub enum ITag {
    /// The pulse is guaranteed/required to last for the exact duration.
    Exact,
    /// The pulse will last at least as long as the given duration.
    Within,
}

/// An interval consists of a type tag, a start time, and a end time.
#[derive(Clone, Hash, PartialEq, Eq)]
pub struct Interval<T>
where
    T: super::TimeRep + Clone,
{
    pub start: T,
    pub end: T,
    pub typ: ITag,
}
impl<T> Interval<T>
where
    T: super::TimeRep + Clone,
{
    /// Construct a [Interval] with `tag` set to [IntervalTime::Exact].
    pub fn exact(start: T, end: T) -> Self {
        Interval {
            start,
            end,
            typ: ITag::Exact,
        }
    }

    pub fn within(start: T, end: T) -> Self {
        Interval {
            start,
            end,
            typ: ITag::Within,
        }
    }

    pub fn resolve(&self, bindings: &HashMap<Id, T>) -> Self {
        Interval {
            start: self.start.resolve(bindings),
            end: self.end.resolve(bindings),
            typ: self.typ.clone(),
        }
    }
}
impl<T> std::fmt::Debug for Interval<T>
where
    T: std::fmt::Debug + Clone + super::TimeRep,
{
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
            IntervalTime::Add { left, right } => SExp(format!(
                "(+ {} {})",
                SExp::from(&**left),
                SExp::from(&**right),
            )),
            IntervalTime::Max { left, right } => SExp(format!(
                "(max {} {})",
                SExp::from(&**left),
                SExp::from(&**right)
            )),
        }
    }
}
