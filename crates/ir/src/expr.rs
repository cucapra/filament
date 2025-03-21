use super::{AddCtx, Component, Ctx, ExprIdx, MutCtx, ParamIdx};
use crate::{Param, Prop, PropIdx, construct_binop};
use fil_ast as ast;
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
        op: ast::Fn,
        args: Vec<ExprIdx>,
    },
    If {
        cond: PropIdx,
        then: ExprIdx,
        alt: ExprIdx,
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
            Expr::If { cond, then, alt } => {
                write!(f, "if {} then {} else {}", cond, then, alt)
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

    pub fn as_param(&self, ctx: &impl Ctx<Expr>) -> Option<ParamIdx> {
        if let Expr::Param(pidx) = ctx.get(*self) {
            Some(*pidx)
        } else {
            None
        }
    }

    #[inline]
    /// Returns the concrete value represented by this expression or errors out.
    /// If an optional value is desired, use [Self::as_concrete] instead.
    pub fn concrete(self, comp: &Component) -> u64 {
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
    pub fn is_param(self, ctx: &impl Ctx<Expr>, param: ParamIdx) -> bool {
        if let Expr::Param(p) = ctx.get(self) {
            *p == param
        } else {
            false
        }
    }

    pub fn pow2(self, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Fn {
            op: ast::Fn::Pow2,
            args: vec![self],
        })
    }

    pub fn log2(self, ctx: &mut impl AddCtx<Expr>) -> Self {
        ctx.add(Expr::Fn {
            op: ast::Fn::Log2,
            args: vec![self],
        })
    }

    /// creates an [Expr::Bin] given two [ExprIdx]s and an [ast::Op].
    fn bin(self, rhs: Self, op: ast::Op) -> Expr {
        Expr::Bin { op, lhs: self, rhs }
    }
}

/// Queries over [ExprIdx]
impl ExprIdx {
    pub fn valid<C>(&self, ctx: &C) -> bool
    where
        C: Ctx<Expr> + Ctx<Prop> + MutCtx<Param>,
    {
        self.relevant_vars(ctx).iter().all(|p| ctx.valid(*p))
    }

    pub fn relevant_props<C>(&self, ctx: &C) -> Vec<PropIdx>
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        let mut props = Vec::new();
        self.relevant_props_acc(ctx, &mut props);
        props
    }

    pub fn relevant_props_acc<C>(&self, ctx: &C, props: &mut Vec<PropIdx>)
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        if let Expr::If { cond, .. } = ctx.get(*self) {
            props.push(*cond);
            let inner_props = cond.relevant_props(ctx);
            props.extend(inner_props);
        }
    }

    pub fn relevant_vars<C>(&self, ctx: &C) -> Vec<ParamIdx>
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        let mut params = Vec::new();
        self.relevant_vars_acc(ctx, &mut params);
        params
    }

    pub fn relevant_vars_acc<C>(&self, ctx: &C, params: &mut Vec<ParamIdx>)
    where
        C: Ctx<Expr> + Ctx<Prop>,
    {
        match ctx.get(*self) {
            Expr::Param(p) => {
                params.push(*p);
            }
            Expr::Concrete(_) => (),
            Expr::Bin { lhs, rhs, .. } => {
                lhs.relevant_vars_acc(ctx, params);
                rhs.relevant_vars_acc(ctx, params);
            }
            Expr::Fn { args, .. } => {
                for e in args {
                    e.relevant_vars_acc(ctx, params);
                }
            }
            Expr::If { cond, then, alt } => {
                cond.relevant_vars_if_acc(ctx, params);
                then.relevant_vars_acc(ctx, params);
                alt.relevant_vars_acc(ctx, params);
            }
        }
    }
}

// creates the binary operator constructors for all [ast::Op] variants.
construct_binop!(
    <impl AddCtx<Expr>>(ExprIdx::bin, ExprIdx) => Expr;
    add = fil_ast::Op::Add;
    sub = fil_ast::Op::Sub;
    mul = fil_ast::Op::Mul;
    div = fil_ast::Op::Div;
    rem = fil_ast::Op::Mod;
);
