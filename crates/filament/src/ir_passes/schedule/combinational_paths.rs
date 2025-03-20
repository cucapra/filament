use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils::{self as utils, AttrCtx};
use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use ordered_float::NotNan;
use std::collections::HashSet;

/// Combinational paths in a component
#[derive(Clone)]
pub struct CombDataflow {
    /// Combinational delays of the component
    delays: ir::DenseIndexInfo<ir::Port, NotNan<f64>>,
    /// Edges in the dataflow graph
    edges: ir::DenseIndexInfo<ir::Port, HashSet<ir::PortIdx>>,
}

impl CombDataflow {
    pub fn new(comp: &ir::Component, ctx: &ir::Context) -> Self {
        let mut edges: ir::DenseIndexInfo<ir::Port, HashSet<_>> =
            ir::DenseIndexInfo::with_default(comp.ports().len());
        let mut delays: ir::DenseIndexInfo<ir::Port, NotNan<f64>> =
            ir::DenseIndexInfo::with_default(comp.ports().len());

        for cmd in &comp.cmds {
            match cmd {
                ir::Command::Invoke(idx) => {
                    let inputs = idx.inputs(comp);
                    let outputs = idx.outputs(comp).collect_vec();

                    for &output in &outputs {
                        let ir::Port {
                            owner: ir::PortOwner::Inv { base, .. },
                            ..
                        } = comp.get(output)
                        else {
                            unreachable!(
                                "Port {} was not an invocation port",
                                comp.display(output)
                            );
                        };

                        // add the combinational delay to the output ports
                        let foreign_idx = base.owner();

                        delays.insert(output, NotNan::new(
                                base.apply(
                                    |foreign_port, foreign_comp| {
                                        *foreign_comp
                                            .port_attrs
                                            .get(foreign_port)
                                            .get(utils::PortFloat::CombDelay)
                                            .unwrap_or_else(
                                                || panic!("Combinational delay not found for port {} in comp {}", foreign_comp.display(foreign_port), ctx.display(foreign_idx))
                                            )
                                        },
                                        ctx,
                                    )
                                )
                                .expect(
                                    "Combinational delays should not be NaN",
                                ));
                    }

                    for src in inputs {
                        for &dst in &outputs {
                            edges.get_mut(src).insert(dst);
                        }
                    }
                }
                ir::Command::Connect(ir::Connect { src, dst, .. }) => {
                    assert!(
                        src.is_port(comp) && dst.is_port(comp),
                        "Bundles should be resolved before constructing dataflow"
                    );

                    edges.get_mut(src.port).insert(dst.port);
                }
                ir::Command::BundleDef(_)
                | ir::Command::ForLoop(_)
                | ir::Command::If(_) => {
                    unreachable!(
                        "Components should be monomorphic and bundle-free"
                    )
                }
                ir::Command::Instance(_)
                | ir::Command::Let(_)
                | ir::Command::Fact(_)
                | ir::Command::Exists(_) => {}
            }
        }

        Self { delays, edges }
    }

    fn critical_paths_rec(
        &self,
        path: LinkedHashSet<ir::PortIdx>,
        len: f64,
    ) -> Vec<Vec<ir::PortIdx>> {
        let current = path.iter().last().copied().unwrap();

        if len >= 1.0 {
            // This path is critical here, we can return it immediately
            return vec![path.iter().cloned().collect()];
        }

        let mut chain = vec![];

        for &succ in self.edges.get(current).iter() {
            let delay: f64 = (*self.delays.get(succ)).into();
            if path.contains(&succ) {
                // We already visited this node, skip it
                continue;
            }

            let mut new_path = path.clone();
            new_path.insert(succ);

            chain.extend(
                self.critical_paths_rec(new_path, len + delay).into_iter(),
            )
        }

        chain
    }
    /// Find the critical paths (sum of edges > clock period) in a DAG using Djikstra
    /// To do so we use DFS to find all paths from every source to every sink
    pub fn critical_paths(&self) -> Vec<Vec<ir::PortIdx>> {
        let mut paths = vec![];
        for (src, _) in self.edges.iter() {
            let mut path = LinkedHashSet::new();

            // Find the combinational delay of this port
            let comb_delay = *self.delays.get(src);

            path.insert(src);
            paths.extend(
                self.critical_paths_rec(path, comb_delay.into()).into_iter(),
            );
        }

        paths
    }
}
