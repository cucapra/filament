use crate as ir;
use itertools::Itertools;
use topological_sort::TopologicalSort;

/// Defines a traversal of the components.
/// Equivalent to the ast traversal except works over IR components.
/// There is an edge between src -> dst if `src` instantiates an instance of `dst`
pub struct Traversal {
    ctx: ir::Context,
    order: Vec<ir::CompIdx>,
}

impl From<ir::Context> for Traversal {
    /// Construct a post-order traversal over a [Context].
    fn from(ctx: ir::Context) -> Self {
        let mut ts = TopologicalSort::<ir::CompIdx>::new();

        // Gets all components that are not primitives.
        let comps = ctx
            .comps
            .iter()
            .filter(|(_, comp)| !comp.is_ext)
            .collect_vec();

        for (idx, _) in comps.iter() {
            ts.insert(*idx);
        }

        for (idx, comp) in comps.iter() {
            for cmd in &comp.cmds {
                Traversal::process_cmd(&ctx, *idx, cmd, &mut ts);
            }
        }

        let order: Vec<_> = ts.collect();
        assert!(
            order.len() == comps.len(),
            "Ordering contains {} elements but context has {} components",
            order.len(),
            comps.len()
        );

        Self { ctx, order }
    }
}

impl Traversal {
    /// Apply a function to each component in a post-order traversal.
    pub fn apply_post_order<F>(self, mut f: F)
    where
        F: FnMut(&ir::Context, ir::CompIdx),
    {
        for idx in self.order.clone() {
            log::trace!("Post-order: {}", idx);
            f(&self.ctx, idx)
        }
    }

    /// Apply a function to each component in a pre-order traversal.
    pub fn apply_pre_order<F>(self, mut f: F)
    where
        F: FnMut(&ir::Context, ir::CompIdx),
    {
        for &idx in self.order.iter().rev() {
            log::trace!("Pre-order: {}", idx);
            f(&self.ctx, idx)
        }
    }

    /// Take the [Context] from the post order structure.
    pub fn take(self) -> ir::Context {
        self.ctx
    }

    fn process_cmd(
        ctx: &ir::Context,
        comp: ir::CompIdx,
        cmd: &ir::Command,
        ts: &mut TopologicalSort<ir::CompIdx>,
    ) {
        match cmd {
            ir::Command::Instance(inst) => {
                let inst = ctx.comps.get(comp).instances().get(*inst);
                // If the instance is not an external, add a dependency edge
                if !ctx.comps.get(inst.comp).is_ext {
                    ts.add_dependency(comp, inst.comp);
                }
            }
            ir::Command::ForLoop(fl) => {
                for cmd in &fl.body {
                    Traversal::process_cmd(ctx, comp, cmd, ts);
                }
            }
            ir::Command::If(i) => {
                for cmd in &i.then {
                    Traversal::process_cmd(ctx, comp, cmd, ts);
                }
                for cmd in &i.alt {
                    Traversal::process_cmd(ctx, comp, cmd, ts);
                }
            }
            ir::Command::Connect(_)
            | ir::Command::BundleDef(_)
            | ir::Command::Invoke(_)
            | ir::Command::Fact(_)
            | ir::Command::Exists(_) => (),
        }
    }
}
