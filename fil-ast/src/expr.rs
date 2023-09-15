use super::{Binding, Id, Loc};
use fil_utils::Error;
use itertools::Itertools;

#[derive(Clone, Copy, PartialEq, Debug)]
/// A concrete value for a parameter, internally either a float or an integer
pub enum Concrete {
    Float(f64),
    UInt(u64),
}

impl Concrete {
    /// Convert a [Concrete] value into a uint
    pub fn u64(self) -> u64 {
        self.try_into().unwrap_or_else(|_| {
            unreachable!("Value {} could not be converted into a uint.", self)
        })
    }

    /// Convert a [Concrete] value into a float
    pub fn f64(self) -> f64 {
        self.try_into().unwrap_or_else(|_| {
            unreachable!("Value {} could not be converted into a float.", self)
        })
    }
}

impl std::fmt::Display for Concrete {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Concrete::Float(n) => write!(f, "{}", n),
            Concrete::UInt(n) => write!(f, "{}", n),
        }
    }
}

impl From<u64> for Concrete {
    fn from(v: u64) -> Self {
        Concrete::UInt(v)
    }
}

impl From<f64> for Concrete {
    fn from(v: f64) -> Self {
        Concrete::Float(v)
    }
}

impl TryFrom<Concrete> for u64 {
    type Error = Error;

    fn try_from(value: Concrete) -> Result<Self, Self::Error> {
        match value {
            Concrete::UInt(n) => Ok(n),
            Concrete::Float(_) => Err(Error::malformed(
                "Cannot concretize float value into uint.",
            )),
        }
    }
}

impl TryFrom<Concrete> for f64 {
    type Error = Error;

    fn try_from(value: Concrete) -> Result<Self, Self::Error> {
        match value {
            Concrete::UInt(n) => Ok(n as f64),
            Concrete::Float(n) => Ok(n),
        }
    }
}

impl std::ops::Add for Concrete {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Concrete::Float(f) => (f + rhs.f64()).into(),
            Concrete::UInt(u) => (u + rhs.u64()).into(),
        }
    }
}

impl std::ops::Sub for Concrete {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        match self {
            Concrete::Float(f) => (f - rhs.f64()).into(),
            Concrete::UInt(u) => (u - rhs.u64()).into(),
        }
    }
}

impl std::ops::Mul for Concrete {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        match self {
            Concrete::Float(f) => (f * rhs.f64()).into(),
            Concrete::UInt(u) => (u * rhs.u64()).into(),
        }
    }
}

impl std::ops::Div for Concrete {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        match self {
            Concrete::Float(f) => (f / rhs.f64()).into(),
            Concrete::UInt(u) => (u / rhs.u64()).into(),
        }
    }
}

impl std::ops::Rem for Concrete {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        match self {
            Concrete::Float(f) => (f % rhs.f64()).into(),
            Concrete::UInt(u) => (u % rhs.u64()).into(),
        }
    }
}

impl std::cmp::PartialOrd for Concrete {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self {
            Concrete::UInt(l) => (*other).try_into().ok().map(|r| l.cmp(&r)),
            Concrete::Float(l) => {
                (*other).try_into().ok().map(|r| {
                    l.partial_cmp(&r).unwrap_or_else(|| {
                        unreachable!(
                            "Values {} and {} could not be compared.", // this should only occur if one of the values is NaN
                            l,
                            other.f64()
                        )
                    })
                })
            }
        }
    }
}

impl std::cmp::Ord for Concrete {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or_else(|| {
            unreachable!(
                "Values {} and {} could not be compared.", // values are different types
                self, other
            )
        })
    }
}

impl Eq for Concrete {}

impl std::hash::Hash for Concrete {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Concrete::Float(f) => f.to_bits().hash(state),
            Concrete::UInt(u) => u.hash(state),
        }
    }
}

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
    Pow2,
    CLog2,
    Pow,
    CLog,
    Sqrt,
    Sin,
    Cos,
    Tan,
    Float,
    Floor,
    Ceil,
    F32Bits,
    F64Bits,
}

impl Fn {
    pub fn eval(self, args: Vec<Concrete>) -> Concrete {
        match (self, &*args) {
            (Fn::Pow2, &[n]) => (1u64 << n.u64()).into(),
            (Fn::CLog2, &[n]) => (n.f64().log2().ceil() as u64).into(),
            (Fn::Pow, &[b, n]) => b.u64().pow(n.u64() as u32).into(),
            (Fn::CLog, &[b, n]) => n.f64().log(b.f64()).ceil().into(),
            (Fn::Sqrt, &[n]) => n.f64().sqrt().into(),
            (Fn::Sin, &[n]) => n.f64().sin().into(),
            (Fn::Cos, &[n]) => n.f64().cos().into(),
            (Fn::Tan, &[n]) => n.f64().tan().into(),
            (Fn::Float, &[n]) => n.f64().into(),
            (Fn::Floor, &[n]) => n.f64().floor().into(),
            (Fn::Ceil, &[n]) => n.f64().ceil().into(),
            (Fn::F32Bits, &[n]) => ((n.f64() as f32).to_bits() as u64).into(),
            (Fn::F64Bits, &[n]) => n.f64().to_bits().into(),
            _ => unreachable!(
                "Function {} did not expect {} arguments.",
                self,
                args.len()
            ),
        }
    }
}

impl std::fmt::Display for Fn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fn::Pow2 => write!(f, "pow2"),
            Fn::Pow => write!(f, "pow"),
            Fn::Sqrt => write!(f, "sqrt"),
            Fn::CLog2 => write!(f, "clog2"),
            Fn::CLog => write!(f, "clog"),
            Fn::Sin => write!(f, "sin"),
            Fn::Cos => write!(f, "cos"),
            Fn::Tan => write!(f, "tan"),
            Fn::Float => write!(f, "float"),
            Fn::Floor => write!(f, "floor"),
            Fn::Ceil => write!(f, "ceil"),
            Fn::F32Bits => write!(f, "f32_bits"),
            Fn::F64Bits => write!(f, "f64_bits"),
        }
    }
}

/// An expression containing integers and abstract variables
#[derive(Clone, Hash, Debug)]
pub enum Expr {
    Concrete(Concrete),
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
}

impl Default for Expr {
    fn default() -> Self {
        Expr::Concrete(0u64.into())
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
            Expr::Concrete(n) => Ok(n.u64()),
            n => Err(Error::malformed(format!("Cannot concretize `{n}'"))),
        }
    }
}

impl Expr {
    /// Construct a new expression from a uint value
    pub fn uint(n: u64) -> Self {
        Expr::Concrete(n.into())
    }

    /// Construct a new expression from a float value
    pub fn float(n: f64) -> Self {
        Expr::Concrete(n.into())
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
        Expr::Op {
            op,
            left: Box::new(l),
            right: Box::new(r),
        }
    }

    /// Resolve this expression using the given binding for abstract variables.
    pub fn resolve(self, bind: &Binding<Expr>) -> Self {
        match self {
            Expr::Concrete(_) | Expr::ParamAccess { .. } => self,
            Expr::Abstract(ref id) => bind.find(id).cloned().unwrap_or(self),
            Expr::App { func, args } => Self::func(
                func,
                args.into_iter().map(|arg| arg.resolve(bind)).collect(),
            ),
            Expr::Op { op, left, right } => {
                let l = left.resolve(bind);
                let r = right.resolve(bind);
                Self::op(op, l, r)
            }
        }
    }
}

impl std::ops::Add for Expr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::op(Op::Add, self, rhs)
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
        Self::op(Op::Sub, self, rhs)
    }
}

impl std::ops::Mul for Expr {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self::op(Op::Mul, self, rhs)
    }
}

impl std::ops::Div for Expr {
    type Output = Self;

    fn div(self, rhs: Self) -> Self::Output {
        Self::op(Op::Div, self, rhs)
    }
}

impl std::ops::Rem for Expr {
    type Output = Self;

    fn rem(self, rhs: Self) -> Self::Output {
        Self::op(Op::Mod, self, rhs)
    }
}

impl std::fmt::Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", ECtx::default().print(self))
    }
}

impl From<Concrete> for Expr {
    fn from(v: Concrete) -> Self {
        Self::Concrete(v)
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
