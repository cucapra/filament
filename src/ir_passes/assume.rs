use crate::{
    ast::{self, FnAssume},
    ir::{self, Ctx, ExprIdx, PropIdx},
    ir_visitor::{Action, Visitor},
    utils::Binding,
};

/// Generates default assumptions to the Filament program for assumptions using custom functions
#[derive(Default)]
pub struct Assume;

impl ir::Component {
    /// Interns new [ir::Expr]s generated from an [ast::Expr] given a binding on [ir::ExprIdx]
    fn add_expr(
        &mut self,
        expr: ast::Expr,
        binding: &Binding<ExprIdx>,
    ) -> ExprIdx {
        match expr {
            ast::Expr::Concrete(v) => self.add(ir::Expr::Concrete(v)),
            ast::Expr::Abstract(id) => {
                *binding.find(&id).unwrap_or_else(|| {
                    panic!(
                        "Missing binding for {id} while interning expression."
                    )
                })
            }
            ast::Expr::Op { op, left, right } => {
                let l = self.add_expr(*left, binding);
                let r = self.add_expr(*right, binding);
                match op {
                    ast::Op::Add => l.add(r, self),
                    ast::Op::Mul => l.mul(r, self),
                    ast::Op::Sub => l.sub(r, self),
                    ast::Op::Div => l.div(r, self),
                    ast::Op::Mod => l.rem(r, self),
                }
            }
            ast::Expr::App { func, arg } => {
                let arg = self.add_expr(*arg, binding);
                match func {
                    ast::UnFn::Pow2 => arg.pow2(self),
                    ast::UnFn::Log2 => arg.log2(self),
                }
            }
        }
    }

    /// Interns a new [ir::Prop] from an [ast::OrderConstraint].
    fn add_orderconstraint(
        &mut self,
        prop: ast::OrderConstraint<ast::Expr>,
        binding: &Binding<ExprIdx>,
    ) -> PropIdx {
        let ast::OrderConstraint { left, right, op } = prop;
        let lhs = self.add_expr(left, binding);
        let rhs = self.add_expr(right, binding);
        self.add(ir::Prop::Cmp(match op {
            ast::OrderOp::Eq => ir::CmpOp::eq(lhs, rhs),
            ast::OrderOp::Gt => ir::CmpOp::gt(lhs, rhs),
            ast::OrderOp::Gte => ir::CmpOp::gte(lhs, rhs),
        }))
    }

    /// Interns a new [ir::Prop] from an [ast::Implication].
    fn add_implication(
        &mut self,
        prop: ast::Implication<ast::Expr>,
        binding: &Binding<ExprIdx>,
    ) -> PropIdx {
        match prop {
            ast::Implication { guard: None, cons } => {
                self.add_orderconstraint(cons, binding)
            }
            ast::Implication {
                guard: Some(g),
                cons,
            } => self
                .add_orderconstraint(g, binding)
                .implies(self.add_orderconstraint(cons, binding), self),
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
                if let Some((op, left, right)) = match (
                    comp.get(*lhs),
                    comp.get(*rhs),
                ) {
                    (ir::Expr::Fn { op, args }, _) => {
                        assert!(
                            args.len() == 1,
                            "Currently Unimplemented: {} requires {} arguments, automatic assumptions only implemented for single argument functions.",
                            op, args.len()
                        );
                        Some((op, *rhs, args[0]))
                    }
                    (_, ir::Expr::Fn { op, args }) => {
                        assert!(
                            args.len() == 1,
                            "Currently Unimplemented: {} requires {} arguments, automatic assumptions only implemented for single argument functions.",
                            op, args.len()
                        );
                        Some((op, *lhs, args[0]))
                    }
                    _ => None,
                } {
                    log::debug!("Generating default assumptions for {p}");
                    // Creates the binding for the left and right [ExprIdx]s
                    let binding = Binding::new(vec![
                        (FnAssume::left(), left),
                        (FnAssume::right(), right),
                    ]);
                    FnAssume::from(op.clone())
                        .assumptions
                        .into_iter()
                        .map(|assumption| {
                            comp.add_implication(assumption, &binding)
                        })
                        .collect()
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
    fn fact(&mut self, f: &mut ir::Fact, comp: &mut ir::Component) -> Action {
        if f.is_assume() {
            Action::AddBefore(
                Assume::prop(f.prop, comp)
                    .into_iter()
                    .filter_map(|prop| comp.assume(prop, f.reason))
                    .collect(),
            )
        } else {
            Action::Continue
        }
    }
}
