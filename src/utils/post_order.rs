use crate::ast;
use std::collections::{HashMap, HashSet};
use topological_sort::{self, TopologicalSort};

/// Defines a traversal of the components.
/// There is an edge between src -> dst if `src` instantiates an instance of `dst`.
pub struct Traversal {
    /// The namespace
    ns: ast::Namespace,
    /// The post-order traversal
    order: Vec<usize>,
}

impl From<ast::Namespace> for Traversal {
    /// Construct a post-order traversal over a namespace.
    fn from(ns: ast::Namespace) -> Self {
        let externs: HashSet<_> =
            ns.externals().map(|(_, sig)| *sig.name.inner()).collect();

        let mut ts = TopologicalSort::<usize>::new();
        let rev_map: HashMap<ast::Id, usize> = ns
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
    cmd: &ast::Command,
    externs: &HashSet<ast::Id>,
    rev_map: &HashMap<ast::Id, usize>,
    ts: &mut TopologicalSort<usize>,
    idx: usize,
) {
    match cmd {
        ast::Command::Instance(inst) => {
            // If the instance is not an external, add a dependency edge
            if !externs.contains(&inst.component) {
                let src = rev_map[&inst.component];
                ts.add_dependency(src, idx);
            }
        }
        ast::Command::ForLoop(fl) => {
            for cmd in &fl.body {
                process_cmd(cmd, externs, rev_map, ts, idx);
            }
        }
        ast::Command::If(i) => {
            for cmd in &i.then {
                process_cmd(cmd, externs, rev_map, ts, idx);
            }
            for cmd in &i.alt {
                process_cmd(cmd, externs, rev_map, ts, idx);
            }
        }
        ast::Command::Connect(_)
        | ast::Command::Invoke(_)
        | ast::Command::Bundle(_)
        | ast::Command::Fact(_)
        | ast::Command::ParamLet(_) => (),
    }
}

impl Traversal {
    /// Apply a mutable function to each component in a post-order traversal.
    pub fn apply_post_order<F>(&mut self, mut upd: F)
    where
        F: FnMut(&mut ast::Component),
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
        F: FnMut(&mut ast::Component),
    {
        for &idx in self.order.iter().rev() {
            let comp = &mut self.ns.components[idx];
            log::trace!("Pre-order: {}", comp.sig.name);
            upd(comp)
        }
    }

    /// Take the namespace from the post order structure,
    /// and returns the ordering of the components
    pub fn take(self) -> (ast::Namespace, Vec<usize>) {
        (self.ns, self.order)
    }
}
