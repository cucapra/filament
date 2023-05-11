use crate::{
    core::{self, Loc, OrderConstraint, Expr},
    utils::Binding,
    passes::Pass
};
use itertools::Itertools;
use std::collections::HashMap;

/// Add default assumptions to the Filament program
pub struct Assume;

impl Assume {
    /// Iterate through the parameter constraints in a signature to see if we can add any default function assumptions
    fn sig(sig: &core::Signature) -> Vec<core::Command> {
        sig.param_constraints
            .iter()
            .filter_map(|c| match c.inner() {
                OrderConstraint {
                    op: core::OrderOp::Eq,
                    left,
                    right
                } => {
                    match (left, right) {
                        (
                            _,
                            Expr::App { func, arg: right}
                        ) => Some (func.clone().assume(left, right)),
                        (
                            Expr::App { func, arg: left},
                            _
                        ) => Some (func.clone().assume(right, left)),
                        _ => None
                    }
                },
                _ => None
            })
            .flatten()
            .map(core::Command::from)
            .collect_vec()
    }

    fn component(comp: core::Component) -> core::Component {
        let mut pre_cmds = Assume::sig(&comp.sig);
        pre_cmds.extend(comp.body.into_iter());

        core::Component {
            body: pre_cmds,
            ..comp
        }
    }
}

impl Pass for Assume
{
    /// Monomorphize the program by generate a component for each parameter of each instance.
    fn transform(ns: core::Namespace) -> core::Namespace {
        core::Namespace {
            components: ns.components
                .into_iter()
                .map(Assume::component)
                .collect_vec(),
            ..ns
        }
    }
}
