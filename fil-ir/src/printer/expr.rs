use super::DisplayCtx;
use crate::{self as ir, Ctx};
use fil_ast as ast;
use itertools::Itertools;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
/// Track the current context within an expression for pretty printing
pub enum ECtx {
    #[default]
    /// Inside an addition priority expression (+ or -)
    Add,
    /// Substraction priority expression (-)
    Sub,
    /// Inside an multiplication priority expression (* or / or %)
    Mul,
}

impl From<ast::Op> for ECtx {
    fn from(op: ast::Op) -> Self {
        match op {
            ast::Op::Add => ECtx::Add,
            ast::Op::Sub => ECtx::Sub,
            ast::Op::Mul | ast::Op::Div | ast::Op::Mod => ECtx::Mul,
        }
    }
}

// Ordering for expression printing context.
// If `self > other`, then that means that `self` binds tigher than `other`.
impl Ord for ECtx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        match (self, other) {
            // Mults are next
            (ECtx::Mul, ECtx::Mul) => Equal,
            (ECtx::Mul, _) => Greater,
            // Subs are next
            (ECtx::Sub, ECtx::Sub) => Equal,
            (ECtx::Sub, ECtx::Mul) => Less,
            (ECtx::Sub, _) => Greater,
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

fn display_expr_helper(
    expr: ir::ExprIdx,
    ctx: ECtx,
    comp: &ir::Component,
) -> String {
    match comp.get(expr) {
        ir::Expr::Param(p) => comp.display(*p),
        ir::Expr::Concrete(n) => format!("{n}"),
        ir::Expr::Bin { op, lhs, rhs } => {
            let inner = ECtx::from(*op);
            let left = display_expr_helper(*lhs, inner, comp);
            let right = display_expr_helper(*rhs, inner, comp);
            // If context binds more tightly than the inner operator,
            // wrap the inner expression in parens.
            if ctx > inner {
                format!("({}{}{})", left, op, right)
            } else {
                format!("{}{}{}", left, op, right)
            }
        }
        ir::Expr::Fn { op, args } => {
            let fn_str = match op {
                ast::Fn::Pow2 => "pow2",
                ast::Fn::Log2 => "log2",
                ast::Fn::SinB => "sin_bits",
                ast::Fn::CosB => "cos_bits",
            };
            format!(
                "{fn_str}({args})",
                args = args
                    .iter()
                    .map(|a| display_expr_helper(*a, ECtx::default(), comp))
                    .join(", ")
            )
        }
    }
}

impl DisplayCtx<ir::ExprIdx> for ir::Component {
    fn write(
        &self,
        idx: ir::ExprIdx,
        f: &mut impl std::fmt::Write,
    ) -> std::fmt::Result {
        let out = display_expr_helper(idx, ECtx::default(), self);
        write!(f, "{}", out)
    }
}
