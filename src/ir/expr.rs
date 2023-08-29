use super::{
    AddCtx, Cmp, CmpOp, Component, Ctx, ExprIdx, ParamIdx, Prop, PropIdx,
};
use crate::ast;
use std::fmt::Display;

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
        args: Vec<ExprIdx>,
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
    /// If the coercion should panic on failure, use [Self::concrete] instead.
    pub fn as_concrete(&self, ctx: &impl Ctx<Expr>) -> Option<u64> {
        if let Expr::Concrete(c) = ctx.get(*self) {
            Some(*c)
        } else {
            None
        }
    }

    #[inline]
    /// Returns the concrete value represented by this expression or errors out.
    /// If an optional value is desired, use [Self::as_concrete] instead.
    pub fn concrete(&self, comp: &Component) -> u64 {
        let Some(c) = self.as_concrete(comp) else {
            comp.internal_error(format!("{} is not a concrete number", self))
        };
        c
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
    pub fn add(self, other: ExprIdx, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Bin {
            op: ast::Op::Add,
            lhs: self,
            rhs: other,
        })
    }

    pub fn mul(self, other: ExprIdx, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Bin {
            op: ast::Op::Mul,
            lhs: self,
            rhs: other,
        })
    }

    pub fn sub(self, other: ExprIdx, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Bin {
            op: ast::Op::Sub,
            lhs: self,
            rhs: other,
        })
    }

    pub fn div(self, other: ExprIdx, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Bin {
            op: ast::Op::Div,
            lhs: self,
            rhs: other,
        })
    }

    pub fn rem(self, other: ExprIdx, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Bin {
            op: ast::Op::Mod,
            lhs: self,
            rhs: other,
        })
    }

    pub fn pow2(self, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Fn {
            op: ast::UnFn::Pow2,
            args: vec![self],
        })
    }

    pub fn log2(self, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Fn {
            op: ast::UnFn::Log2,
            args: vec![self],
        })
    }

    /// The proposition `self > other`
    pub fn gt<C>(&self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: AddCtx<Prop> + Ctx<Expr>,
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
        C: AddCtx<Prop> + Ctx<Expr>,
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
        C: AddCtx<Prop> + Ctx<Expr>,
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
        C: AddCtx<Prop> + Ctx<Expr>,
    {
        other.gt(self, ctx)
    }

    pub fn lte<C>(self, other: ExprIdx, ctx: &mut C) -> PropIdx
    where
        C: AddCtx<Prop> + Ctx<Expr>,
    {
        other.gte(self, ctx)
    }
}
