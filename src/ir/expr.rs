use super::{Ctx, ExprIdx, ParamIdx};
use crate::ast;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
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
}
