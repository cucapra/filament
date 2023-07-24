use std::{collections::HashMap, fmt::Display};

use super::{Cmp, CmpOp, Ctx, ExprIdx, ParamIdx, Prop, PropIdx};
use crate::ast;

#[derive(PartialEq, Eq, Hash, Clone)]
pub enum Expr {
    Param(ParamIdx),
    Concrete(u64),
    Bin {
        op: ast::Op,
        lhs: ExprIdx,
        rhs: ExprIdx,
    },
    Fn {
        op: ast::UnFn,
        args: Box<[ExprIdx]>,
    },
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Param(p) => write!(f, "{}", p),
            Expr::Concrete(c) => write!(f, "{}", c),
            Expr::Bin { op, lhs, rhs } => write!(f, "{} {} {}", lhs, op, rhs),
            Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|arg| format!("{}", arg))
                    .collect::<Vec<_>>()
                    .join(", ");
                write!(f, "{}({})", op, args)
            }
        }
    }
}

impl ExprIdx {
    #[inline]
    /// Attempts to convert this expression into a concrete value.
    pub fn as_concrete(&self, ctx: &impl Ctx<Expr>) -> Option<u64> {
        if let Expr::Concrete(c) = ctx.get(*self) {
            Some(*c)
        } else {
            None
        }
    }

    pub fn resolve(
        self,
        ctx: &mut impl Ctx<Expr>,
        binding: HashMap<ParamIdx, u64>,
    ) -> ExprIdx {
        match ctx.get(self) {
            Expr::Param(p) => ctx.add(Expr::Concrete(*binding.get(p).unwrap())),
            Expr::Concrete(_) => self,
            Expr::Bin { op, lhs, rhs } => {
                let lhs = lhs.resolve(ctx, binding);
                let rhs = rhs.resolve(ctx, binding);
                match op {
                    ast::Op::Add => lhs.add(rhs, ctx),
                    ast::Op::Sub => lhs.sub(rhs, ctx),
                    ast::Op::Mul => lhs.mul(rhs, ctx),
                    ast::Op::Div => lhs.div(rhs, ctx),
                    ast::Op::Mod => lhs.rem(rhs, ctx),
                }
            }
            Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|arg| arg.resolve(ctx, binding))
                    .collect::<Vec<_>>();
                match op {
                    ast::UnFn::Pow2 => args[0].pow2(ctx),
                    ast::UnFn::Log2 => args[0].log2(ctx),
                }
            }
        }
    }

    /// Returns true if this expression is a constant.
    /// Note that this process *does not* automatically reduce the expression.
    /// For example, `1 + 1` is not going to be reduced to `2`.
    pub fn is_const(&self, ctx: &impl Ctx<Expr>, n: u64) -> bool {
        self.as_concrete(ctx).map(|c| c == n).unwrap_or(false)
    }

    /// Returns true of the expression is equal to the given parameter.
    pub fn is_param(&self, ctx: &impl Ctx<Expr>, param: ParamIdx) -> bool {
        if let Expr::Param(p) = ctx.get(*self) {
            *p == param
        } else {
            false
        }
    }

    /// Adds two expressions together.
    pub fn add(self, other: ExprIdx, ctx: &mut impl Ctx<Expr>) -> Self {
        let l = self.as_concrete(ctx);
        let r = other.as_concrete(ctx);

        match (l, r) {
            (Some(l), Some(r)) => ctx.add(Expr::Concrete(l + r)),
            (None, Some(0)) => self,
            (Some(0), None) => other,
            _ => {
                // Sort the operands to canonicalize the expression
                let (lhs, rhs) = if self < other {
                    (self, other)
                } else {
                    (other, self)
                };
                ctx.add(Expr::Bin {
                    op: ast::Op::Add,
                    lhs,
                    rhs,
                })
            }
        }
    }

    pub fn mul(self, other: ExprIdx, ctx: &mut impl Ctx<Expr>) -> Self {
        let l = self.as_concrete(ctx);
        let r = other.as_concrete(ctx);

        match (l, r) {
            (Some(l), Some(r)) => ctx.add(Expr::Concrete(l * r)),
            (None, Some(0)) | (Some(0), None) => ctx.add(Expr::Concrete(0)),
            (None, Some(1)) => self,
            (Some(1), None) => other,
            _ => {
                // Sort the operands to canonicalize the expression
                let (lhs, rhs) = if self < other {
                    (self, other)
                } else {
                    (other, self)
                };
                ctx.add(Expr::Bin {
                    op: ast::Op::Mul,
                    lhs,
                    rhs,
                })
            }
        }
    }

    pub fn sub(self, other: ExprIdx, ctx: &mut impl Ctx<Expr>) -> Self {
        if self == other {
            return ctx.add(Expr::Concrete(0));
        }
        let l = self.as_concrete(ctx);
        let r = other.as_concrete(ctx);

        match (l, r) {
            (Some(l), Some(r)) if l > r => ctx.add(Expr::Concrete(l - r)),
            (None, Some(0)) => self,
            _ => ctx.add(Expr::Bin {
                op: ast::Op::Sub,
                lhs: self,
                rhs: other,
            }),
        }
    }

    pub fn div(self, other: ExprIdx, ctx: &mut impl Ctx<Expr>) -> Self {
        if self == other {
            return ctx.add(Expr::Concrete(1));
        }

        let l = self.as_concrete(ctx);
        let r = other.as_concrete(ctx);

        match (l, r) {
            (Some(l), Some(r)) => ctx.add(Expr::Concrete(l / r)),
            (None, Some(1)) => self,
            _ => ctx.add(Expr::Bin {
                op: ast::Op::Div,
                lhs: self,
                rhs: other,
            }),
        }
    }

    pub fn rem(self, other: ExprIdx, ctx: &mut impl Ctx<Expr>) -> Self {
        if self == other {
            return ctx.add(Expr::Concrete(0));
        }

        let l = self.as_concrete(ctx);
        let r = other.as_concrete(ctx);

        match (l, r) {
            (Some(l), Some(r)) => ctx.add(Expr::Concrete(l % r)),
            (None, Some(1)) => ctx.add(Expr::Concrete(0)),
            _ => ctx.add(Expr::Bin {
                op: ast::Op::Mod,
                lhs: self,
                rhs: other,
            }),
        }
    }

    pub fn pow2(self, ctx: &mut impl Ctx<Expr>) -> Self {
        let l = self.as_concrete(ctx);

        match l {
            Some(l) => ctx.add(Expr::Concrete(1 << l)),
            _ => ctx.add(Expr::Fn {
                op: ast::UnFn::Pow2,
                args: Box::new([self]),
            }),
        }
    }

    pub fn log2(self, ctx: &mut impl Ctx<Expr>) -> Self {
        let l = self.as_concrete(ctx);

        match l {
            Some(l) => ctx.add(Expr::Concrete(l.trailing_zeros() as u64)),
            _ => ctx.add(Expr::Fn {
                op: ast::UnFn::Log2,
                args: Box::new([self]),
            }),
        }
    }

    /// The proposition `self > other`
    pub fn gt<C>(&self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        if let (Some(l), Some(r)) =
            (self.as_concrete(ctx), other.as_concrete(ctx))
        {
            if l > r {
                ctx.add(Prop::True)
            } else {
                ctx.add(Prop::False)
            }
        } else {
            ctx.add(Prop::Cmp(CmpOp {
                op: Cmp::Gt,
                lhs: *self,
                rhs: other,
            }))
        }
    }

    pub fn gte<C>(&self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        if let (Some(l), Some(r)) =
            (self.as_concrete(ctx), other.as_concrete(ctx))
        {
            if l >= r {
                ctx.add(Prop::True)
            } else {
                ctx.add(Prop::False)
            }
        } else {
            ctx.add(Prop::Cmp(CmpOp {
                op: Cmp::Gte,
                lhs: *self,
                rhs: other,
            }))
        }
    }

    pub fn equal<C>(&self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        if let (Some(l), Some(r)) =
            (self.as_concrete(ctx), other.as_concrete(ctx))
        {
            if l == r {
                ctx.add(Prop::True)
            } else {
                ctx.add(Prop::False)
            }
        } else if self == &other {
            return ctx.add(Prop::True);
        } else {
            ctx.add(Prop::Cmp(CmpOp {
                op: Cmp::Eq,
                lhs: *self,
                rhs: other,
            }))
        }
    }

    pub fn lt<C>(self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        other.gt(self, ctx)
    }

    pub fn lte<C>(self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        other.gte(self, ctx)
    }
}
