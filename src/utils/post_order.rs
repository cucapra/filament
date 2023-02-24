use crate::core;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use topological_sort::{self, TopologicalSort};

/// Defines a post-order traversal of the components.
/// There is an edge between src -> dst if `src` instantiates an instance of `dst`.
pub struct PostOrder {
    /// The namespace
    ns: core::Namespace,
    /// The post-order traversal
    order: TopologicalSort<usize>,
}

impl From<core::Namespace> for PostOrder {
    /// Construct a post-order traversal over a namespace.
    fn from(ns: core::Namespace) -> Self {
        let externs: HashSet<_> =
            ns.signatures().map(|(_, sig)| sig.name.clone()).collect();

        let mut ts = TopologicalSort::<usize>::new();
        let rev_map: HashMap<core::Id, usize> = ns
            .components
            .iter()
            .enumerate()
            .map(|(idx, comp)| (comp.sig.name.clone(), idx))
            .collect();

        for (idx, comp) in ns.components.iter().enumerate() {
            for inst in &comp.body {
                process_cmd(inst, &externs, &rev_map, &mut ts, idx);
            }
        }

        Self { ns, order: ts }
    }
}

fn process_cmd(
    cmd: &core::Command,
    externs: &HashSet<core::Id>,
    rev_map: &HashMap<core::Id, usize>,
    ts: &mut TopologicalSort<usize>,
    idx: usize,
) {
    match cmd {
        core::Command::Instance(inst) => {
            // If the instance is not an external, add a dependency edge
            if !externs.contains(&inst.component) {
                let src = rev_map[&inst.component];
                ts.add_dependency(idx, src);
            }
        }
        core::Command::ForLoop(fl) => {
            for cmd in &fl.body {
                process_cmd(cmd, externs, rev_map, ts, idx);
            }
        }
        core::Command::Connect(_)
        | core::Command::Invoke(_)
        | core::Command::Bundle(_)
        | core::Command::Fsm(_) => (),
    }
}

impl PostOrder {
    /// Apply a mutable function to each component in the post-order traversal.
    pub fn apply_post_order<F>(&mut self, mut upd: F)
    where
        F: FnMut(&mut core::Component),
    {
        for idx in self.order.clone() {
            let comp = &mut self.ns.components[idx];
            log::trace!("Post-order: {}", comp.sig.name);
            upd(comp)
        }
    }

    /// Apply a function to each component in the pre-order traversal.
    pub fn apply_pre_order<F>(&mut self, mut upd: F)
    where
        F: FnMut(&mut core::Component),
    {
        let order = self.order.clone().into_iter().collect_vec();
        for idx in order.into_iter().rev() {
            let comp = &mut self.ns.components[idx];
            log::trace!("Pre-order: {}", comp.sig.name);
            upd(comp)
        }
    }

    /// Take the namespace from the post order structure.
    pub fn take(self) -> core::Namespace {
        self.ns
    }
}
