use super::{Id, Time};
use crate::utils::{self, SExp};
use itertools::Itertools;
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd)]
pub enum Op {
    Add,
    Sub,
    Mul,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
        }
    }
}

#[derive(Clone, Hash)]
pub enum Expr {
    Concrete(u64),
    Abstract(Id),
    Op {
        op: Op,
        left: Box<Expr>,
        right: Box<Expr>,
    },
}

impl Default for Expr {
    fn default() -> Self {
        Expr::Concrete(0)
    }
}

impl TryFrom<Expr> for u64 {
    type Error = ();

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        match value {
            Expr::Concrete(n) => Ok(n),
            _ => Err(()),
        }
    }
}
impl TryFrom<&Expr> for u64 {
    type Error = ();

    fn try_from(value: &Expr) -> Result<Self, Self::Error> {
        match value {
            Expr::Concrete(n) => Ok(*n),
            _ => Err(()),
        }
    }
}

impl Expr {
    /// Construct a new expression from a concrete value
    pub fn concrete(n: u64) -> Self {
        Expr::Concrete(n)
    }

    /// Resolve this expression using the given binding for abstract variables.
    pub fn resolve(self, bind: &utils::Binding<Expr>) -> Self {
        todo!()
    }

    /// Check if this TimeSum is equal to 0
    pub fn is_zero(&self) -> bool {
        matches!(self, Expr::Concrete(0))
    }

    /// Get all the abstract variables in this expression
    pub fn exprs(&self) -> &Vec<Id> {
        todo!()
    }

    /// Attempt to simplify this expression
    pub fn simplify(self) -> SimplifiedExpr {
        todo!()
    }
}

impl std::ops::Add for Expr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::Op {
            op: Op::Add,
            left: Box::new(self),
            right: Box::new(rhs),
        }
    }
}

impl std::ops::AddAssign for Expr {
    fn add_assign(&mut self, rhs: Self) {
        todo!()
    }
}

impl std::ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::Op {
            op: Op::Sub,
            left: Box::new(self),
            right: Box::new(rhs),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Concrete(n) => write!(f, "{}", n),
            Expr::Abstract(id) => write!(f, "{}", id),
            Expr::Op { op, left, right } => {
                write!(f, "({} {} {})", op, left, right)
            }
        }
    }
}

impl From<Expr> for utils::SExp {
    fn from(value: Expr) -> Self {
        todo!()
        // utils::SExp(format!(
        //     "(+ {} {})",
        //     value.abs.iter().map(|t| t.as_ref()).join(" "),
        //     value.concrete
        // ))
    }
}

impl From<Vec<u64>> for Expr {
    fn from(v: Vec<u64>) -> Self {
        Self::concrete(v.iter().sum())
    }
}

impl From<u64> for Expr {
    fn from(v: u64) -> Self {
        Self::concrete(v)
    }
}

impl From<Id> for Expr {
    fn from(v: Id) -> Self {
        Self::Abstract(v)
    }
}

impl From<Time> for SExp {
    fn from(t: Time) -> SExp {
        todo!()
        // if t.offset().abs.is_empty() && t.offset().concrete == 0 {
        //     SExp(format!("{}", t.event))
        // } else if t.offset().abs.is_empty() {
        //     SExp(format!("(+ {} {})", t.event, t.offset().concrete))
        // } else {
        //     SExp(format!(
        //         "(+ {} {} {})",
        //         t.event,
        //         t.offset().concrete,
        //         t.offset()
        //             .abs
        //             .iter()
        //             .map(|e| e.to_string())
        //             .collect_vec()
        //             .join(" ")
        //     ))
        // }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
/// An expression that has been simplified and canonicalized.
/// Supports efficient comparison and ordering operations.
pub struct SimplifiedExpr;

impl Display for SimplifiedExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}
