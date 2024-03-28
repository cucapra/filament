use super::{DisplayCtx, IOResult};
use crate::{
    self as ir,
    printer::prop::{display_prop_helper, PCtx},
    Ctx,
};
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
            format!(
                "{op}({args})",
                args = args
                    .iter()
                    .map(|a| display_expr_helper(*a, ECtx::default(), comp))
                    .join(", ")
            )
        }
        ir::Expr::If { cond, then, alt } => {
            let pctx = match comp.get(*cond) {
                ir::Prop::True | ir::Prop::False => PCtx::default(),
                ir::Prop::Cmp(_)
                | ir::Prop::TimeCmp(_)
                | ir::Prop::TimeSubCmp(_) => PCtx::Cmp,
                ir::Prop::Not(_) => PCtx::Not,
                ir::Prop::And(_, _) => PCtx::And,
                ir::Prop::Or(_, _) => PCtx::Or,
                ir::Prop::Implies(_, _) => PCtx::Implies,
            };

            let cond = display_prop_helper(*cond, pctx, comp);
            let then = display_expr_helper(*then, ctx, comp);
            let alt = display_expr_helper(*alt, ctx, comp);
            format!("if {cond} then {then} else {alt}")
        }
    }
}

impl DisplayCtx<ir::ExprIdx> for ir::Component {
    fn write(&self, idx: ir::ExprIdx, f: &mut impl std::io::Write) -> IOResult {
        let out = display_expr_helper(idx, ECtx::default(), self);
        write!(f, "{}", out)
    }
}
