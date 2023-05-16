use crate::{ast, passes::Pass};
use itertools::Itertools;

/// Add default assumptions to the Filament program
pub struct Assume;

impl Assume {
    /// Iterate through the parameter constraints in a signature to see if we can add any default function assumptions
    fn sig(sig: &ast::Signature) -> Vec<ast::Command> {
        sig.param_constraints
            .iter()
            .flat_map(|c| ast::Fact::from_constraint(c.inner()))
            .map(ast::Command::from)
            .collect_vec()
    }

    fn component(comp: ast::Component) -> ast::Component {
        let mut pre_cmds = Assume::sig(&comp.sig);
        pre_cmds.extend(comp.body.into_iter());

        ast::Component {
            body: pre_cmds,
            ..comp
        }
    }
}

impl Pass for Assume {
    /// Monomorphize the program by generate a component for each parameter of each instance.
    fn transform(ns: ast::Namespace) -> ast::Namespace {
        ast::Namespace {
            components: ns
                .components
                .into_iter()
                .map(Assume::component)
                .collect_vec(),
            ..ns
        }
    }
}
