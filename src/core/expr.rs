use super::Id;
use crate::{
    errors,
    utils::{self, SExp},
};
use std::fmt::Display;

/// Binary operation over expressions
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
            Op::Mod => write!(f, "%"),
        }
    }
}

impl From<Op> for utils::SExp {
    fn from(value: Op) -> Self {
        match value {
            Op::Add => SExp("+".to_string()),
            Op::Sub => SExp("-".to_string()),
            Op::Mul => SExp("*".to_string()),
            Op::Div => SExp("div".to_string()),
            Op::Mod => SExp("mod".to_string()),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd)]
/// A unary uninterpreted function over integers.
pub enum UnFn {
    /// The `pow2` function
    Pow2,
    /// The `log2` function
    Log2,
}
impl std::fmt::Display for UnFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnFn::Pow2 => write!(f, "pow2"),
            UnFn::Log2 => write!(f, "log2"),
        }
    }
}
impl UnFn {
    pub fn apply(self, arg: Expr) -> Expr {
        match (self, arg) {
            (UnFn::Pow2, Expr::Concrete(n)) => {
                Expr::Concrete(2u64.pow(n as u32))
            }
            (UnFn::Log2, Expr::Concrete(n)) => {
                Expr::Concrete((n as f64).log2().ceil() as u64)
            }
            (func, arg) => Expr::App {
                func,
                arg: Box::new(arg),
            },
        }
    }
}

/// An expression containing integers and abstract variables
#[derive(Clone, Hash)]
pub enum Expr {
    Concrete(u64),
    Abstract(Id),
    App {
        func: UnFn,
        arg: Box<Expr>,
    },
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
    type Error = errors::Error;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}
impl TryFrom<&Expr> for u64 {
    type Error = errors::Error;

    fn try_from(value: &Expr) -> Result<Self, Self::Error> {
        match value {
            Expr::Concrete(n) => Ok(*n),
            n => Err(errors::Error::malformed(format!(
                "Cannot concretize `{n}'"
            ))),
        }
    }
}

impl Expr {
    /// Construct a new expression from a concrete value
    pub fn concrete(n: u64) -> Self {
        Expr::Concrete(n)
    }

    /// Construct a new expression from an abstract variable
    pub fn abs(id: Id) -> Self {
        Expr::Abstract(id)
    }

    /// Function application
    pub fn func(func: UnFn, arg: Expr) -> Self {
        func.apply(arg)
    }

    pub fn op(op: Op, l: Expr, r: Expr) -> Self {
        match op {
            Op::Add => l + r,
            Op::Sub => l - r,
            Op::Mul => l * r,
            Op::Div => l / r,
            Op::Mod => l % r,
        }
    }

    fn op_base(op: Op, l: Expr, r: Expr) -> Self {
        Expr::Op {
            op,
            left: Box::new(l),
            right: Box::new(r),
        }
    }

    /// Resolve this expression using the given binding for abstract variables.
    pub fn resolve(self, bind: &utils::Binding<Expr>) -> Self {
        match self {
            Expr::Concrete(_) => self,
            Expr::Abstract(ref id) => bind.find(id).cloned().unwrap_or(self),
            Expr::App { func, arg } => func.apply(arg.resolve(bind)),
            Expr::Op { op, left, right } => {
                let l = left.resolve(bind);
                let r = right.resolve(bind);
                match op {
                    Op::Add => l + r,
                    Op::Sub => l - r,
                    Op::Mul => l * r,
                    Op::Div => l / r,
                    Op::Mod => l % r,
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
            Expr::App { arg, .. } => Box::new(arg.exprs()),
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
            (Expr::Concrete(0), e) | (e, Expr::Concrete(0)) => e,
            (Expr::Concrete(l), Expr::Concrete(r)) => Expr::Concrete(l + r),
            (left, right) => Self::op_base(Op::Add, left, right),
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
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (lhs, Expr::Concrete(0)) => lhs,
            (Expr::Concrete(l), Expr::Concrete(r)) => match l.checked_sub(r) {
                Some(n) => Expr::Concrete(n),
                None => Self::op_base(Op::Sub, l.into(), r.into()),
            },
            (left, right) => Self::op_base(Op::Sub, left, right),
        }
    }
}

impl std::ops::Mul for Expr {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Expr::Concrete(0), _) | (_, Expr::Concrete(0)) => {
                Expr::Concrete(0)
            }
            (Expr::Concrete(1), e) | (e, Expr::Concrete(1)) => e,
            (Expr::Concrete(l), Expr::Concrete(r)) => Expr::Concrete(l * r),
            (left, right) => Self::op_base(Op::Mul, left, right),
        }
    }
}

impl std::ops::Div for Expr {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Expr::Concrete(0), _) => Expr::Concrete(0),
            (e, Expr::Concrete(1)) => e,
            (Expr::Concrete(l), Expr::Concrete(r)) => Expr::Concrete(l / r),
            (left, right) => Self::op_base(Op::Div, left, right),
        }
    }
}

impl std::ops::Rem for Expr {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (Expr::Concrete(0), _) | (_, Expr::Concrete(1)) => {
                Expr::Concrete(0)
            }
            (Expr::Concrete(l), Expr::Concrete(r)) => Expr::Concrete(l % r),
            (left, right) => Self::op_base(Op::Mod, left, right),
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ECtx::Func.print(self))
        // match self {
        //     Expr::Concrete(n) => write!(f, "{}", n),
        //     Expr::Abstract(id) => write!(f, "#{}", id),
        //     Expr::App { func, arg } => write!(f, "{}({})", func, arg),
        //     Expr::Op { op, left, right } => {
        //         write!(f, "({left}{op}{right})")
        //     }
        // }
    }
}

impl From<Expr> for utils::SExp {
    fn from(value: Expr) -> Self {
        match value {
            Expr::Concrete(n) => SExp(format!("{}", n)),
            Expr::Abstract(id) => SExp(format!("{}", id)),
            Expr::App { func, arg } => {
                SExp(format!("({} {})", func, SExp::from(*arg)))
            }
            Expr::Op { op, left, right } => SExp(format!(
                "({} {} {})",
                SExp::from(op),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Track the current context within an expression for pretty printing
enum ECtx {
    /// Inside an addition priority expression (+ or -)
    Add,
    /// Inside an multiplication priority expression (* or / or %)
    Mul,
    /// Inside a function application
    Func,
}

impl ECtx {
    fn print(&self, e: &Expr) -> String {
        match e {
            Expr::Concrete(n) => {
                format!("{n}")
            }
            Expr::Abstract(v) => {
                format!("#{v}")
            }
            Expr::App { func, arg } => {
                format!("{}({})", func, Self::Func.print(arg))
            }
            Expr::Op { op, left, right } => {
                let inner = Self::from(*op);
                let left = inner.print(left);
                let right = inner.print(right);
                if inner < *self {
                    format!("({}{}{})", left, op, right)
                } else {
                    format!("{}{}{}", left, op, right)
                }
            }
        }
    }
}

impl From<Op> for ECtx {
    fn from(op: Op) -> Self {
        match op {
            Op::Add | Op::Sub => ECtx::Add,
            Op::Mul | Op::Div | Op::Mod => ECtx::Mul,
        }
    }
}

// Ordering for expression printing context. If other is less than this,
// then we are in a tightly binding context and need to add parens.
impl Ord for ECtx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        match (self, other) {
            // Functions are the tightest
            (ECtx::Func, ECtx::Func) => Equal,
            (ECtx::Func, _) => Greater,
            // Mults are next
            (ECtx::Mul, ECtx::Mul) => Equal,
            (ECtx::Mul, ECtx::Func) => Less,
            (ECtx::Mul, _) => Greater,
            // Adds are last
            (ECtx::Add, ECtx::Add) => Equal,
            (ECtx::Add, _) => Less,
        }
    }
}

impl PartialOrd for ECtx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
