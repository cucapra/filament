use crate::ast_visitor::{Action, Visitor};
use fil_ast as ast;

/// Loads the scheduling model into attributes
#[derive(Default)]
pub struct DesugarUnknowns;

impl DesugarUnknowns {
    /// Split a time expression into a let
    fn maybe_unknown(
        &self,
        mu: &mut ast::MaybeUnknown,
        name: ast::Id,
    ) -> Option<ast::ParamLet> {
        if mu.is_unknown() {
            // Create a new let binding for the event variable
            let inner_name = ast::Loc::unknown(name);

            let mut offset = ast::MaybeUnknown::Known(ast::Expr::Abstract(
                inner_name.clone(),
            ));

            std::mem::swap(mu, &mut offset);

            Some(ast::ParamLet {
                name: inner_name,
                expr: offset,
            })
        } else {
            None
        }
    }
}

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
                    // Create a new let binding for the event variable
                    let inner_name =
                        ast::Id::new(format!("__{}_schedule{}", inv.name, i));

                    i += 1;

                    self.maybe_unknown(
                        &mut abstract_var.inner_mut().offset,
                        inner_name,
                    )
                })
                .map(ast::Command::from)
                .collect(),
        )
    }

    fn exists(&mut self, x: &mut ast::Exists) -> Action {
        let ast::Exists { param, bind } = x;

        let inner_name = ast::Id::new(format!("__{}_schedule", param));

        match self.maybe_unknown(bind, inner_name) {
            Some(v) => Action::AddBefore(vec![v.into()]),
            None => Action::Continue,
        }
    }

    fn bundle(&mut self, bun: &mut fil_ast::Bundle) -> Action {
        let ast::Range { start, end } = bun.typ.liveness.inner_mut();

        let start_name =
            ast::Id::new(format!("__{}_schedule_start", bun.name.inner()));

        let end_name =
            ast::Id::new(format!("__{}_schedule_end", bun.name.inner()));

        let start_let = self.maybe_unknown(&mut start.offset, start_name);
        let end_let = self.maybe_unknown(&mut end.offset, end_name);

        if start_let.is_some() || end_let.is_some() {
            Action::AddBefore(
                start_let
                    .into_iter()
                    .chain(end_let)
                    .map(ast::Command::from)
                    .collect(),
            )
        } else {
            Action::Continue
        }
    }
}
