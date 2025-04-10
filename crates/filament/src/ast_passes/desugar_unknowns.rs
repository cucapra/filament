use crate::ast_visitor::{Action, Visitor};
use fil_ast as ast;

/// Loads the scheduling model into attributes
#[derive(Default)]
pub struct DesugarUnknowns;

impl Visitor for DesugarUnknowns {
    fn name() -> &'static str {
        "desugar-eventbinds"
    }

    fn invoke(&mut self, inv: &mut ast::Invoke) -> Action {
        let ast::Invoke { abstract_vars, .. } = inv;

        let mut i = 0;

        // Convert all 'G+? event bindings to a pair of
        // let x = ?
        // and 'G + x.

        Action::AddBefore(
            abstract_vars
                .iter_mut()
                .flat_map(|abstract_var| {
                    if abstract_var.inner().offset.is_none() {
                        // Create a new let binding for the event variable
                        let inner_name = ast::Loc::unknown(ast::Id::new(
                            format!("__{}_schedule{}", inv.name, i),
                        ));

                        i += 1;

                        abstract_var.inner_mut().offset =
                            Some(ast::Expr::Abstract(inner_name.clone()));

                        Some(
                            ast::ParamLet {
                                name: inner_name,
                                expr: None,
                            }
                            .into(),
                        )
                    } else {
                        None
                    }
                })
                .collect(),
        )
    }

    fn exists(&mut self, x: &mut ast::Exists) -> Action {
        let ast::Exists { param, bind } = x;

        if bind.inner().is_none() {
            // Create a new let binding for the event variable
            let inner_name = ast::Loc::unknown(ast::Id::new(format!(
                "__{}_schedule",
                param
            )));

            *bind.inner_mut() = Some(ast::Expr::Abstract(inner_name.clone()));

            Action::AddBefore(vec![
                ast::ParamLet {
                    name: inner_name,
                    expr: None,
                }
                .into(),
            ])
        } else {
            Action::Continue
        }
    }
}
