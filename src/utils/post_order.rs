use crate::core;
use std::collections::{HashMap, HashSet};
use topological_sort::{self, TopologicalSort};

/// Defines a traversal of the components.
/// There is an edge between src -> dst if `src` instantiates an instance of `dst`.
pub struct Traversal {
    /// The namespace
    ns: core::Namespace,
    /// The post-order traversal
    order: Vec<usize>,
}

impl From<core::Namespace> for Traversal {
    /// Construct a post-order traversal over a namespace.
    fn from(ns: core::Namespace) -> Self {
        let externs: HashSet<_> =
            ns.signatures().map(|(_, sig)| *sig.name.inner()).collect();

        let mut ts = TopologicalSort::<usize>::new();
        let rev_map: HashMap<core::Id, usize> = ns
            .components
            .iter()
            .enumerate()
            .map(|(idx, comp)| {
                // Add component to the graph
                ts.insert(idx);
                (*comp.sig.name.inner(), idx)
            })
            .collect();

        for (idx, comp) in ns.components.iter().enumerate() {
            for inst in &comp.body {
                process_cmd(inst, &externs, &rev_map, &mut ts, idx);
            }
        }

        let order: Vec<_> = ts.collect();
        debug_assert!(
            order.len() == ns.components.len(),
            "Ordering contains {} elements but namespace has {} components",
            order.len(),
            ns.components.len()
        );

        Self { ns, order }
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
        core::Command::If(i) => {
            for cmd in &i.then {
                process_cmd(cmd, externs, rev_map, ts, idx);
            }
            for cmd in &i.alt {
                process_cmd(cmd, externs, rev_map, ts, idx);
            }
        }
        core::Command::Connect(_)
        | core::Command::Invoke(_)
        | core::Command::Bundle(_)
        | core::Command::Fsm(_) => (),
    }
}

impl Traversal {
    /// Apply a mutable function to each component in a post-order traversal.
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

    /// Apply a function to each component in a pre-order traversal.
    pub fn apply_pre_order<F>(&mut self, mut upd: F)
    where
        F: FnMut(&mut core::Component),
    {
        for &idx in self.order.iter().rev() {
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
