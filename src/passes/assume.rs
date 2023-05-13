use crate::{
    core,
    passes::Pass,
};
use itertools::Itertools;

/// Add default assumptions to the Filament program
pub struct Assume;

impl Assume {
    /// Iterate through the parameter constraints in a signature to see if we can add any default function assumptions
    fn sig(sig: &core::Signature) -> Vec<core::Command> {
        sig.param_constraints
            .iter()
            .map(|c| core::Assume::from_constraint(c.inner()))
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

impl Pass for Assume {
    /// Monomorphize the program by generate a component for each parameter of each instance.
    fn transform(ns: core::Namespace) -> core::Namespace {
        core::Namespace {
            components: ns
                .components
                .into_iter()
                .map(Assume::component)
                .collect_vec(),
            ..ns
        }
    }
}
