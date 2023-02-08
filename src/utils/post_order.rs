use crate::core::{self, TimeRep, WidthRep};
use std::collections::{HashMap, HashSet};
use topological_sort::{self, TopologicalSort};

/// Defines a post-order traversal of the components.
/// There is an edge between src -> dst if `src` instantiates an instance of `dst`.
pub struct PostOrder<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    /// The namespace
    ns: core::Namespace<T, W>,
    /// The post-order traversal
    order: TopologicalSort<usize>,
}

impl<T, W> From<core::Namespace<T, W>> for PostOrder<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    /// Construct a post-order traversal over a namespace.
    fn from(ns: core::Namespace<T, W>) -> Self {
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
                if let core::Command::Instance(inst) = inst {
                    // If the instance is not an external, add a dependency edge
                    if !externs.contains(&inst.component) {
                        let src = rev_map[&inst.component];
                        ts.add_dependency(idx, src);
                    }
                }
            }
        }

        Self { ns, order: ts }
    }
}

impl<T, W> PostOrder<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    /// Apply a mutable function to each component in the post-order traversal.
    pub fn apply<F>(&mut self, mut upd: F)
    where
        F: FnMut(&mut core::Component<T, W>),
    {
        self.order
            .clone()
            .into_iter()
            .for_each(|idx| upd(&mut self.ns.components[idx]))
    }

    /// Take the namespace from the post order structure.
    pub fn take(self) -> core::Namespace<T, W> {
        self.ns
    }
}
