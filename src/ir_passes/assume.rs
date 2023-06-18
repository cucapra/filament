use crate::{
    ast::{self, FnAssume},
    ir::{self, Ctx, ExprIdx, PropIdx},
    ir_visitor::{Action, Visitor},
    utils::Binding,
};

#[derive(Default)]
pub struct Assume;

impl ir::Component {
    /// Interns a new [ir::Expr] from an [ast::Expr] given a binding on [ir::ExprIdx]
    fn add_expr(
        &mut self,
        expr: ast::Expr,
        binding: &Binding<ExprIdx>,
    ) -> ExprIdx {
        match expr {
            ast::Expr::Concrete(v) => self.add(ir::Expr::Concrete(v)),
            ast::Expr::Abstract(id) => *binding.find(&id).expect(&format!(
                "Missing binding for {id} while interning expression."
            )),
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

    fn add_orderconstraint(
        &mut self,
        prop: ast::OrderConstraint<ast::Expr>,
        binding: &Binding<ExprIdx>,
    ) -> PropIdx {
        match prop {
            ast::OrderConstraint { left, right, op } => {
                let lhs = self.add_expr(left, binding);
                let rhs = self.add_expr(right, binding);
                self.add(ir::Prop::Cmp(match op {
                    ast::OrderOp::Eq => ir::CmpOp::eq(lhs, rhs),
                    ast::OrderOp::Gt => ir::CmpOp::gt(lhs, rhs),
                    ast::OrderOp::Gte => ir::CmpOp::gte(lhs, rhs),
                }))
            }
        }
    }

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

impl Visitor for Assume {
    fn fact(&mut self, f: &mut ir::Fact, comp: &mut ir::Component) -> Action {
        let props = comp.props();
        if f.is_assert() {
            match props.get(f.prop) {
                ir::Prop::Cmp(ir::CmpOp {
                    op: ir::Cmp::Eq,
                    lhs,
                    rhs,
                }) => {
                    let exprs = comp.exprs();
                    let funcmatch = match (exprs.get(*lhs), exprs.get(*rhs)) {
                        (ir::Expr::Fn { op, args }, _) => {
                            debug_assert!(args.len() == 1, "Currently Unimplemented: {} requires {} arguments, automatic assumptions only implemented for single argument functions.", op, args.len());
                            Some((
                                op,
                                Binding::new(vec![
                                    (FnAssume::left(), *rhs),
                                    (FnAssume::right(), args[0]),
                                ]),
                            ))
                        }
                        (_, ir::Expr::Fn { op, args }) => {
                            debug_assert!(args.len() == 1, "Currently Unimplemented: {} requires {} arguments, automatic assumptions only implemented for single argument functions.", op, args.len());
                            Some((
                                op,
                                Binding::new(vec![
                                    (FnAssume::left(), *lhs),
                                    (FnAssume::right(), args[0]),
                                ]),
                            ))
                        }
                        _ => None,
                    };

                    match funcmatch {
                        None => Action::Continue,
                        Some((op, binding)) => Action::AddBefore(
                            FnAssume::from(op.clone())
                                .assumptions
                                .into_iter()
                                .filter_map(|assumption| {
                                    let imp = comp.add_implication(
                                        assumption, &binding,
                                    );
                                    comp.assert(
                                        imp,
                                        f.reason,
                                    )
                                })
                                .collect(),
                        ),
                    }
                }
                _ => Action::Continue,
            }
        } else {
            Action::Continue
        }
    }
}
