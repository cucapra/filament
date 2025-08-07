use crate::OrderConstraint;
use crate::OrderOp;

use super::{Binding, Id, Loc};
use fil_utils::Error;
use itertools::Itertools;

/// Binary operation over expressions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl std::fmt::Display for Op {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd)]
/// A unary uninterpreted function over integers.
pub enum Fn {
    /// The `pow2` function
    Pow2,
    /// The `log2` function
    Log2,
    /// Returns the 32 bit floating point bits of the sine
    SinB,
    CosB,
    /// Bit reverse the given integer
    BitRev,
}
impl std::fmt::Display for Fn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fn::Pow2 => write!(f, "pow2"),
            Fn::Log2 => write!(f, "log2"),
            Fn::SinB => write!(f, "sin_bits"),
            Fn::CosB => write!(f, "cos_bits"),
            Fn::BitRev => write!(f, "bit_rev"),
        }
    }
}

impl Fn {
    pub fn eval(self, args: Vec<u64>) -> u64 {
        match (self, &*args) {
            (Fn::Pow2, &[n]) => 2u64.pow(n as u32),
            (Fn::Log2, &[n]) => (n as f64).log2().ceil() as u64,
            (Fn::SinB, &[num, den]) => {
                ((2. * std::f64::consts::PI * (num as f64) / (den as f64)).sin()
                    as f32)
                    .to_bits() as u64
            }
            (Fn::CosB, &[num, den]) => {
                ((2. * std::f64::consts::PI * (num as f64) / (den as f64)).cos()
                    as f32)
                    .to_bits() as u64
            }
            (Fn::BitRev, &[n, numbits]) => {
                let mut n = n;
                let mut rev = 0;
                for _ in 0..numbits {
                    rev <<= 1;
                    rev |= n & 1;
                    n >>= 1;
                }
                rev
            }
            _ => unreachable!(
                "Function {} did not expect {} arguments.",
                self,
                args.len()
            ),
        }
    }
}

/// An expression containing integers and abstract variables
#[derive(Clone, Hash, Debug)]
pub enum Expr {
    Concrete(u64),
    Abstract(Loc<Id>),
    ParamAccess {
        inst: Loc<Id>,
        param: Loc<Id>,
    },
    App {
        func: Fn,
        args: Vec<Expr>,
    },
    Op {
        op: Op,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    If {
        cond: OrderConstraint<Box<Expr>>,
        then: Box<Expr>,
        alt: Box<Expr>,
    },
}

impl Default for Expr {
    fn default() -> Self {
        Expr::Concrete(0)
    }
}

impl TryFrom<Expr> for u64 {
    type Error = Error;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        (&value).try_into()
    }
}

impl TryFrom<&Expr> for u64 {
    type Error = Error;

    fn try_from(value: &Expr) -> Result<Self, Self::Error> {
        match value {
            Expr::Concrete(n) => Ok(*n),
            n => Err(Error::malformed(format!("Cannot concretize `{n}'"))),
        }
    }
}

impl Expr {
    /// Construct a new expression from a concrete value
    pub fn concrete(n: u64) -> Self {
        Expr::Concrete(n)
    }

    /// Construct a new expression from an abstract variable
    pub fn abs(id: Loc<Id>) -> Self {
        Expr::Abstract(id)
    }

    /// Function application
    pub fn func(func: Fn, args: Vec<Expr>) -> Self {
        Expr::App { func, args }
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

    pub fn if_expr(cond: OrderConstraint<Expr>, then: Expr, alt: Expr) -> Self {
        let cond = OrderConstraint {
            left: Box::new(cond.left),
            right: Box::new(cond.right),
            op: cond.op,
        };
        Expr::If {
            cond,
            then: Box::new(then),
            alt: Box::new(alt),
        }
    }

    /// Resolve this expression using the given binding for abstract variables.
    pub fn resolve(self, bind: &Binding<Expr>) -> Self {
        match self {
            Expr::Concrete(_) | Expr::ParamAccess { .. } => self,
            Expr::Abstract(ref id) => bind.find(id).cloned().unwrap_or(self),
            Expr::App { func, args } => Expr::App {
                func,
                args: args.into_iter().map(|arg| arg.resolve(bind)).collect(),
            },
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
            Expr::If { cond, then, alt } => {
                // let OrderConstraint {left, right, op} = cond;
                // let l = Box::new(left.resolve(bind));
                // let r = Box::new(right.resolve(bind));
                // let cond = OrderConstraint{left: l, right: r, op};
                let cond = cond.resolve_expr(bind);
                let then = Box::new(then.resolve(bind));
                let alt = Box::new(alt.resolve(bind));
                Expr::If { cond, then, alt }
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

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ECtx::default().print(self))
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
        Self::Abstract(Loc::unknown(v))
    }
}

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
/// Track the current context within an expression for pretty printinga
enum ECtx {
    #[default]
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
                format!("{v}")
            }
            Expr::ParamAccess { inst, param } => {
                format!("{inst}::{param}")
            }
            Expr::App { func, args } => {
                format!(
                    "{}({})",
                    func,
                    args.iter().map(|arg| Self::Func.print(arg)).join(", ")
                )
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
            Expr::If { cond, then, alt } => {
                let cond_l = self.print(&cond.left);
                let cond_r = self.print(&cond.right);
                let op = match &cond.op {
                    OrderOp::Gt => ">",
                    OrderOp::Gte => ">=",
                    OrderOp::Eq => "=",
                    OrderOp::Lt => "<",
                    OrderOp::Lte => "<=",
                    OrderOp::Neq => "!=",
                };
                let then = self.print(then);
                let alt = self.print(alt);
                format!(
                    "if {} {} {} {{{}}} else {{{}}}",
                    cond_l, op, cond_r, then, alt
                )
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
