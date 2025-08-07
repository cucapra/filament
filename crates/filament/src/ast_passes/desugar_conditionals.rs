use crate::ast_visitor::{Action, Visitor};
use fil_ast as ast;
use fil_ast::Loc;
use fil_utils::{Error, FilamentResult};

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
            match self.desugar_conditional_constraint(conditional_constraint) {
                Ok(facts) => new_facts.extend(facts),
                Err(err) => {
                    // Report error immediately and exit
                    eprintln!(
                        "[ERROR] Conditional constraint desugaring failed:"
                    );
                    eprintln!("  - {}", err.kind);
                    std::process::exit(1);
                }
            }
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
    ) -> FilamentResult<Vec<ast::Command>> {
        use ast::FCons;

        match &**constraint {
            FCons::ConditionalC {
                condition,
                then_constraint,
                else_constraint,
            } => {
                let mut facts = Vec::new();

                // Create: assume condition => then_constraint
                let then_fact = self.create_assume_fact(condition.clone(), then_constraint)?;
                facts.push(ast::Command::Fact(then_fact));

                // Create: assume !condition => else_constraint
                let negated_condition = self.negate_condition(condition)?;
                let else_fact = self.create_assume_fact(negated_condition, else_constraint)?;
                facts.push(ast::Command::Fact(else_fact));

                Ok(facts)
            }
            _ => {
                Err(Error::misc(
                    "Internal error: expected conditional constraint but found other constraint type".to_string()
                ))
            }
        }
    }

    fn create_assume_fact(
        &self,
        condition: ast::OrderConstraint<ast::Expr>,
        constraint: &ast::FCons,
    ) -> FilamentResult<ast::Fact> {
        // Convert FCons to OrderConstraint
        let constraint_oc = self.fcons_to_order_constraint(constraint)?;

        // Create implication: condition => constraint
        let implication = ast::Implication::implies(condition, constraint_oc);

        // Create assume fact
        Ok(ast::Fact::assume(Loc::unknown(implication)))
    }

    fn fcons_to_order_constraint(
        &self,
        fcons: &ast::FCons,
    ) -> FilamentResult<ast::OrderConstraint<ast::Expr>> {
        use ast::FCons;

        match fcons {
            FCons::ExprC(oc) => Ok(oc.clone()),
            FCons::TimeC(_) => {
                Err(Error::misc(
                    "Time constraints in conditional expressions are not supported. \
                     Conditional constraints must use parameter expressions only.".to_string()
                ))
            }
            FCons::ConditionalC { condition: _, then_constraint: _, else_constraint: _ } => {
                // Handle nested conditional constraints recursively
                // Convert C1 ? (C2 ? A : B) : C into multiple assumes

                // For nested conditionals, we need to create a chain of implications
                // This is complex, so for now we'll expand them into separate constraints

                // Create a temporary condition for the nested constraint
                // This requires generating fresh variables, which is complex
                // For now, return a descriptive error

                Err(Error::misc(
                    "Nested conditional constraints are not yet supported. \
                     Please flatten your conditional constraints into separate where clauses.".to_string()
                ))
            }
        }
    }

    fn negate_condition(
        &self,
        condition: &ast::OrderConstraint<ast::Expr>,
    ) -> FilamentResult<ast::OrderConstraint<ast::Expr>> {
        use ast::OrderOp;

        // Handle all comparison operators
        match condition.op {
            OrderOp::Gt => {
                // !(a > b) becomes (a <= b)
                Ok(ast::OrderConstraint::lte(
                    condition.left.clone(),
                    condition.right.clone(),
                ))
            }
            OrderOp::Gte => {
                // !(a >= b) becomes (a < b)
                Ok(ast::OrderConstraint::lt(
                    condition.left.clone(),
                    condition.right.clone(),
                ))
            }
            OrderOp::Lt => {
                // !(a < b) becomes (a >= b)
                Ok(ast::OrderConstraint::gte(
                    condition.left.clone(),
                    condition.right.clone(),
                ))
            }
            OrderOp::Lte => {
                // !(a <= b) becomes (a > b)
                Ok(ast::OrderConstraint::gt(
                    condition.left.clone(),
                    condition.right.clone(),
                ))
            }
            OrderOp::Neq => {
                // !(a != b) becomes (a == b)
                Ok(ast::OrderConstraint::eq(
                    condition.left.clone(),
                    condition.right.clone(),
                ))
            }
            OrderOp::Eq => {
                // !(a == b) becomes (a != b)
                Ok(ast::OrderConstraint::neq(
                    condition.left.clone(),
                    condition.right.clone(),
                ))
            }
        }
    }
}
