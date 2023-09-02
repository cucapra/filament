use super::DisplayCtx;
use crate::ir::{self, Ctx};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Context to track proposition bindings
pub enum PCtx {
    Not,
    Cmp,
    And,
    Or,
    Implies,
}

impl Ord for PCtx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        use PCtx::*;
        match (self, other) {
            // Negations
            (Not, Not) => Equal,
            (Not, _) => Greater,
            // Comparisons
            (Cmp, Cmp) => Equal,
            (Cmp, Not) => Less,
            (Cmp, _) => Greater,
            // Conjunctions
            (And, And) => Equal,
            (And, Not | Cmp) => Less,
            (And, _) => Greater,
            // Disjunctions
            (Or, Or) => Equal,
            (Or, Not | And | Cmp) => Less,
            (Or, _) => Greater,
            // Implications
            (Implies, Implies) => Equal,
            (Implies, _) => Less,
        }
    }
}

impl PartialOrd for PCtx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

fn display_cmp<T>(
    cmp: &ir::CmpOp<T>,
    ctx: PCtx,
    print_base: impl Fn(T) -> String,
) -> String
where
    T: Clone,
{
    let ir::CmpOp { op, lhs, rhs } = cmp;
    let l = print_base(lhs.clone());
    let r = print_base(rhs.clone());
    if ctx > PCtx::Cmp {
        format!("({} {} {})", l, op, r)
    } else {
        format!("{} {} {}", l, op, r)
    }
}

fn display_prop_helper(
    prop: ir::PropIdx,
    ctx: PCtx,
    comp: &ir::Component,
) -> String {
    match comp.get(prop) {
        ir::Prop::True => "true".to_string(),
        ir::Prop::False => "false".to_string(),
        ir::Prop::Cmp(c) => display_cmp(c, ctx, |e| comp.display(e)),
        ir::Prop::TimeCmp(cmp) => display_cmp(cmp, ctx, |t| comp.display(t)),
        ir::Prop::TimeSubCmp(cmp) => {
            display_cmp(cmp, ctx, |t| comp.display(&t))
        }
        ir::Prop::Not(p) => {
            format!("!{}", display_prop_helper(*p, PCtx::Not, comp))
        }
        ir::Prop::And(l, r) => {
            let inner = PCtx::And;
            let l = display_prop_helper(*l, inner, comp);
            let r = display_prop_helper(*r, inner, comp);
            if inner < ctx {
                format!("({} & {})", l, r)
            } else {
                format!("{} & {}", l, r)
            }
        }
        ir::Prop::Or(l, r) => {
            let inner = PCtx::Or;
            let l = display_prop_helper(*l, inner, comp);
            let r = display_prop_helper(*r, inner, comp);
            if inner < ctx {
                format!("({} | {})", l, r)
            } else {
                format!("{} | {}", l, r)
            }
        }
        ir::Prop::Implies(l, r) => {
            let inner = PCtx::Implies;
            let l = display_prop_helper(*l, inner, comp);
            let r = display_prop_helper(*r, inner, comp);
            if inner < ctx {
                format!("({} => {})", l, r)
            } else {
                format!("{} => {}", l, r)
            }
        }
    }
}

impl DisplayCtx<ir::PropIdx> for ir::Component {
    fn write(
        &self,
        val: ir::PropIdx,
        f: &mut impl std::fmt::Write,
    ) -> std::fmt::Result {
        let out = display_prop_helper(val, PCtx::Implies, self);
        write!(f, "{out}")
    }
}
