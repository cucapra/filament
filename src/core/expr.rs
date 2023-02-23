use super::Id;
use crate::{
    expr_simplifier,
    utils::{self, SExp},
};
use std::fmt::Display;

/// Binary operation over expressions
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
        }
    }
}

/// An expression containing integers and abstract variables
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
    type Error = String;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}
impl TryFrom<&Expr> for u64 {
    type Error = String;

    fn try_from(value: &Expr) -> Result<Self, Self::Error> {
        match value {
            Expr::Concrete(n) => Ok(*n),
            n => Err(format!("`{n}'")),
        }
    }
}

impl Expr {
    /// Construct a new expression from a concrete value
    pub fn concrete(n: u64) -> Self {
        Expr::Concrete(n)
    }

    pub fn op(op: Op, left: Expr, right: Expr) -> Self {
        let out = Expr::Op {
            op,
            left: Box::new(left),
            right: Box::new(right),
        };
        // expr_simplifier::Simplifier::simplify(
        //     &SExp::from(out.clone()).to_string(),
        // );
        out
    }

    /// Resolve this expression using the given binding for abstract variables.
    pub fn resolve(self, bind: &utils::Binding<Expr>) -> Self {
        match self {
            Expr::Concrete(_) => self,
            Expr::Abstract(ref id) => bind.find(id).cloned().unwrap_or(self),
            Expr::Op { op, left, right } => {
                let left = left.resolve(bind);
                let right = right.resolve(bind);
                Expr::Op {
                    op,
                    left: Box::new(left),
                    right: Box::new(right),
                }
            }
        }
    }

    /// Check if this TimeSum is equal to 0
    pub fn is_zero(&self) -> bool {
        matches!(self, Expr::Concrete(0))
    }

    /// Get all the abstract variables in this expression
    pub fn exprs(&self) -> Box<dyn Iterator<Item = &Id> + '_> {
        match self {
            Expr::Concrete(_) => Box::new(std::iter::empty()),
            Expr::Abstract(id) => Box::new(std::iter::once(id)),
            Expr::Op { left, right, .. } => {
                Box::new(left.exprs().chain(right.exprs()))
            }
        }
    }
}

impl std::ops::Add for Expr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Expr::Concrete(0), rhs) => rhs,
            (lhs, Expr::Concrete(0)) => lhs,
            (Expr::Concrete(l), Expr::Concrete(r)) => Expr::Concrete(l + r),
            (left, right) => Self::op(Op::Add, left, right),
        }
    }
}

impl std::ops::AddAssign for Expr {
    fn add_assign(&mut self, rhs: Self) {
        let lhs = std::mem::take(self);
        *self = lhs + rhs;
    }
}

impl std::ops::Sub for Expr {
    type Output = Expr;

    fn sub(self, rhs: Self) -> Self::Output {
        Self::op(Op::Sub, self, rhs)
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Concrete(n) => write!(f, "{}", n),
            Expr::Abstract(id) => write!(f, "#{}", id),
            Expr::Op { op, left, right } => {
                write!(f, "({left}{op}{right})")
            }
        }
    }
}

impl From<Expr> for utils::SExp {
    fn from(value: Expr) -> Self {
        match value {
            Expr::Concrete(n) => SExp(format!("{}", n)),
            Expr::Abstract(id) => SExp(format!("{}", id)),
            Expr::Op { op, left, right } => SExp(format!(
                "({} {} {})",
                op,
                SExp::from(*left),
                SExp::from(*right)
            )),
        }
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
