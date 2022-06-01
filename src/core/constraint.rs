use std::collections::HashMap;

use crate::interval_checking::SExp;

use super::{FsmIdxs, Id, TimeRep};

/// Ordering operator for constraints
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum OrderOp {
    Gt,
    Lt,
    Eq,
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
