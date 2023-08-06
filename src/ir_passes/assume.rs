use crate::{
    ast,
    ir::{self, Ctx, ExprIdx, PropIdx},
    ir_visitor::{Action, Visitor, VisitorData},
};

/// Generates default assumptions to the Filament program for assumptions using custom functions
#[derive(Default)]
pub struct Assume;

impl ir::Component {
    /// Adds the assumptions associated with a proposition of the form `#l = f(#r)` to the component.
    fn add_assumptions(
        &mut self,
        f: ast::UnFn,
        lhs: ExprIdx,
        rhs: ExprIdx,
    ) -> Vec<PropIdx> {
        // Define constant expressions used
        let zero = self.add(ir::Expr::Concrete(0));
        let one = self.add(ir::Expr::Concrete(1));
        let two = self.add(ir::Expr::Concrete(2));

        match f {
            ast::UnFn::Pow2 => vec![
                // #l * 2 = pow2(#r + 1)
                lhs.mul(two, self)
                    .equal(rhs.add(one, self).pow2(self), self),
                // #r >= 1 => #l = pow2(#r - 1)*2
                rhs.gte(one, self).implies(
                    lhs.equal(
                        rhs.sub(one, self).pow2(self).mul(two, self),
                        self,
                    ),
                    self,
                ),
                // #l = 1 => #r = 0
                lhs.equal(one, self).implies(rhs.equal(zero, self), self),
                // #r = 0 => #l = 1
                rhs.equal(zero, self).implies(lhs.equal(one, self), self),
            ],
            ast::UnFn::Log2 => vec![
                // #l + 1 = log2(#r * 2)
                lhs.add(one, self)
                    .equal(rhs.mul(two, self).log2(self), self),
                // #l >= 1 => (#l - 1 = log2(#r / 2)) & ((#r / 2) * 2 = #r)
                lhs.gte(one, self).implies(
                    lhs.sub(one, self)
                        .equal(rhs.div(two, self).log2(self), self)
                        .and(
                            rhs.div(two, self).mul(two, self).equal(rhs, self),
                            self,
                        ),
                    self,
                ),
                // #l = 0 => #r = 1
                lhs.equal(zero, self).implies(rhs.equal(one, self), self),
                // #r = 1 => #l = 0
                rhs.equal(one, self).implies(lhs.equal(zero, self), self),
            ],
        }
    }
}

impl Assume {
    /// Checks a proposition for whether it matches the form `#l = f(#r)` for some custom function `f`. Additionally recurses on `&` chains.
    /// Generates the assumptions associated with each [ast::UnFn] and returns a list of [ir::Prop]s for each.
    /// TODO: Implement assumption generation for functions taking more than one argument.
    fn prop(p: ir::PropIdx, comp: &mut ir::Component) -> Vec<PropIdx> {
        let p = comp.get(p);
        match p {
            ir::Prop::Cmp(ir::CmpOp {
                op: ir::Cmp::Eq,
                lhs,
                rhs,
            }) => {
                // Matches over the cases `op(args) = rhs` and `lhs = op(args)` to
                // define the `op`, `left`, and `right` for the equivalent equation `left = op(right)`
                if let Some((op, lhs, rhs)) = match (
                    comp.get(*lhs),
                    comp.get(*rhs),
                ) {
                    (ir::Expr::Fn { op, args }, _) => {
                        assert!(
                            args.len() == 1,
                            "Currently Unimplemented: {} requires {} arguments, automatic assumptions only implemented for single argument functions.",
                            op, args.len()
                        );
                        Some((*op, *rhs, args[0]))
                    }
                    (_, ir::Expr::Fn { op, args }) => {
                        assert!(
                            args.len() == 1,
                            "Currently Unimplemented: {} requires {} arguments, automatic assumptions only implemented for single argument functions.",
                            op, args.len()
                        );
                        Some((*op, *lhs, args[0]))
                    }
                    _ => None,
                } {
                    log::debug!("Generating default assumptions for {p}");
                    comp.add_assumptions(op, lhs, rhs)
                } else {
                    vec![]
                }
            }
            // Recurse on both the left and right subexpressions if the proposition is of the form `l & r`
            ir::Prop::And(lhs, rhs) => {
                let (lhs, rhs) = (*lhs, *rhs);
                Assume::prop(lhs, comp)
                    .into_iter()
                    .chain(Assume::prop(rhs, comp).into_iter())
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
