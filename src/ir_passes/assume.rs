use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ast as ast;
use fil_ir::{self as ir, Ctx, ExprIdx, PropIdx};

/// Generates default assumptions to the Filament program for assumptions using custom functions
#[derive(Default)]
pub struct Assume;

impl Assume {
    /// Adds the assumptions associated with a proposition of the form `#l = f(#r)` to the component.
    fn add_assumptions(
        ctx: &mut ir::Component,
        f: ast::Fn,
        lhs: ExprIdx,
        args: Vec<ExprIdx>,
    ) -> Vec<PropIdx> {
        // Define constant expressions used
        let zero = ctx.uint(0);
        let one = ctx.uint(1);
        let two = ctx.uint(2);

        match (f, &*args) {
            (ast::Fn::Pow2, &[rhs]) => vec![
                // #l * 2 = pow2(#r + 1)
                lhs.mul(two, ctx).equal(rhs.add(one, ctx).pow2(ctx), ctx),
                // #r >= 1 => #l = pow2(#r - 1)*2
                rhs.gte(one, ctx).implies(
                    lhs.equal(rhs.sub(one, ctx).pow2(ctx).mul(two, ctx), ctx),
                    ctx,
                ),
                // #l = 1 => #r = 0
                lhs.equal(one, ctx).implies(rhs.equal(zero, ctx), ctx),
                // #r = 0 => #l = 1
                rhs.equal(zero, ctx).implies(lhs.equal(one, ctx), ctx),
            ],
            (ast::Fn::CLog2, &[rhs]) => vec![
                // #l + 1 = log2(#r * 2)
                lhs.add(one, ctx).equal(rhs.mul(two, ctx).log2(ctx), ctx),
                // #l >= 1 => (#l - 1 = log2(#r / 2)) & ((#r / 2) * 2 = #r)
                lhs.gte(one, ctx).implies(
                    lhs.sub(one, ctx)
                        .equal(rhs.div(two, ctx).log2(ctx), ctx)
                        .and(
                            rhs.div(two, ctx).mul(two, ctx).equal(rhs, ctx),
                            ctx,
                        ),
                    ctx,
                ),
                // #l = 0 => #r = 1
                lhs.equal(zero, ctx).implies(rhs.equal(one, ctx), ctx),
                // #r = 1 => #l = 0
                rhs.equal(one, ctx).implies(lhs.equal(zero, ctx), ctx),
            ],
            (f, args) => {
                unreachable!(
                    "Function {} did not expect {} arguments.",
                    f,
                    args.len()
                );
            }
        }
    }
}

impl Assume {
    /// Checks a proposition for whether it matches the form `#l = f(#r)` for some custom function `f`. Additionally recurses on `&` chains.
    /// Generates the assumptions associated with each [ast::Fn] and returns a list of [ir::Prop]s for each.
    fn prop(p: ir::PropIdx, comp: &mut ir::Component) -> Vec<PropIdx> {
        let p = comp.get(p);
        match p {
            ir::Prop::Cmp(ir::CmpOp {
                op: ir::Cmp::Eq,
                lhs,
                rhs,
            }) => {
                // Matches over the cases `op(..args) = rhs` and `lhs = op(..args)` to
                // define the `op`, `left`, and `args` for the equivalent equation `left = op(args)`
                if let Some((op, lhs, args)) =
                    match (comp.get(*lhs), comp.get(*rhs)) {
                        (ir::Expr::Fn { op, args }, _) => {
                            Some((*op, *rhs, args))
                        }
                        (_, ir::Expr::Fn { op, args }) => {
                            Some((*op, *lhs, args))
                        }
                        _ => None,
                    }
                {
                    log::debug!("Generating default assumptions for {p}");
                    Self::add_assumptions(comp, op, lhs, args.clone())
                } else {
                    vec![]
                }
            }
            // Recurse on both the left and right subexpressions if the proposition is of the form `l & r`
            ir::Prop::And(lhs, rhs) => {
                let (lhs, rhs) = (*lhs, *rhs);
                Assume::prop(lhs, comp)
                    .into_iter()
                    .chain(Assume::prop(rhs, comp))
                    .collect()
            }
            _ => vec![],
        }
    }
}

impl Visitor for Assume {
    fn name() -> &'static str {
        "add-assume"
    }

    fn fact(&mut self, f: &mut ir::Fact, data: &mut VisitorData) -> Action {
        if f.is_assume() {
            Action::AddBefore(
                Assume::prop(f.prop, &mut data.comp)
                    .into_iter()
                    .filter_map(|prop| data.comp.assume(prop, f.reason))
                    .collect(),
            )
        } else {
            Action::Continue
        }
    }
}
