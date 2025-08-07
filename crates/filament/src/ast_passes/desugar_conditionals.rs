use crate::ast_visitor::{Action, Visitor};
use fil_ast as ast;
use fil_ast::Loc;

/// Desugars conditional constraints into assume statements
#[derive(Default)]
pub struct DesugarConditionals;

impl Visitor for DesugarConditionals {
    fn name() -> &'static str {
        "desugar-conditionals"
    }

    fn signature(&mut self, sig: &mut ast::Signature) -> Action {
        // Process conditional constraints and convert them to facts
        let mut new_facts = Vec::new();

        for conditional_constraint in &sig.conditional_constraints {
            let facts =
                self.desugar_conditional_constraint(conditional_constraint);
            new_facts.extend(facts);
        }

        // Clear the conditional constraints since we've processed them
        sig.conditional_constraints.clear();

        if !new_facts.is_empty() {
            Action::AddBefore(new_facts)
        } else {
            Action::Continue
        }
    }
}

impl DesugarConditionals {
    fn desugar_conditional_constraint(
        &self,
        constraint: &Loc<ast::FCons>,
    ) -> Vec<ast::Command> {
        use ast::FCons;

        match &**constraint {
            FCons::ConditionalC {
                condition,
                then_constraint,
                else_constraint,
            } => {
                let mut facts = Vec::new();

                // Create: assume condition => then_constraint
                if let Some(then_fact) =
                    self.create_assume_fact(condition.clone(), then_constraint)
                {
                    facts.push(ast::Command::Fact(then_fact));
                }

                // Create: assume !condition => else_constraint
                if let Some(negated_condition) =
                    self.negate_condition(condition)
                {
                    if let Some(else_fact) = self
                        .create_assume_fact(negated_condition, else_constraint)
                    {
                        facts.push(ast::Command::Fact(else_fact));
                    }
                }

                facts
            }
            _ => {
                // Not a conditional constraint, shouldn't happen but handle gracefully
                vec![]
            }
        }
    }

    fn create_assume_fact(
        &self,
        condition: ast::OrderConstraint<ast::Expr>,
        constraint: &ast::FCons,
    ) -> Option<ast::Fact> {
        // Convert FCons to OrderConstraint
        let constraint_oc = self.fcons_to_order_constraint(constraint)?;

        // Create implication: condition => constraint
        let implication = ast::Implication::implies(condition, constraint_oc);

        // Create assume fact
        Some(ast::Fact::assume(Loc::unknown(implication)))
    }

    fn fcons_to_order_constraint(
        &self,
        fcons: &ast::FCons,
    ) -> Option<ast::OrderConstraint<ast::Expr>> {
        use ast::FCons;

        match fcons {
            FCons::ExprC(oc) => Some(oc.clone()),
            FCons::TimeC(_) => {
                // Cannot convert time constraint to expr constraint
                // This should probably be a compilation error
                None
            }
            FCons::ConditionalC { .. } => {
                // Nested conditional constraints - would need recursive handling
                // For now, unsupported
                None
            }
        }
    }

    fn negate_condition(
        &self,
        condition: &ast::OrderConstraint<ast::Expr>,
    ) -> Option<ast::OrderConstraint<ast::Expr>> {
        use ast::OrderOp;

        // For now, only handle simple negations
        match condition.op {
            OrderOp::Gt => {
                // !(a > b) becomes (a <= b), which is (b >= a)
                Some(ast::OrderConstraint::gte(
                    condition.right.clone(),
                    condition.left.clone(),
                ))
            }
            OrderOp::Gte => {
                // !(a >= b) becomes (a < b), which is (b > a)
                Some(ast::OrderConstraint::gt(
                    condition.right.clone(),
                    condition.left.clone(),
                ))
            }
            OrderOp::Eq => {
                // !(a == b) is more complex - we can't easily represent != in the constraint system
                // For now, return None to indicate we can't negate this
                None
            }
        }
    }
}
