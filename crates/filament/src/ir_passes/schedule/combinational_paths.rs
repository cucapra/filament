use fil_ir::{self as ir, DisplayCtx};
use fil_utils::{self as utils, AttrCtx};
use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use ordered_float::NotNan;

/// Combinational paths in a component
#[derive(Clone)]
pub struct CombDataflow<'comp> {
    /// The component
    comp: &'comp ir::Component,
    /// Edges in the dataflow graph
    edges:
        ir::DenseIndexInfo<ir::Port, ir::SparseInfoMap<ir::Port, NotNan<f64>>>,
}

impl<'comp> From<&'comp ir::Component> for CombDataflow<'comp> {
    fn from(comp: &'comp ir::Component) -> Self {
        let mut edges: ir::DenseIndexInfo<ir::Port, ir::SparseInfoMap<_, _>> =
            ir::DenseIndexInfo::with_default(comp.ports().len());

        for cmd in &comp.cmds {
            match cmd {
                ir::Command::Invoke(idx) => {
                    let inputs = idx.inputs(comp);
                    let outputs = idx
                        .outputs(comp)
                        .map(|output| {
                            (
                                output,
                                NotNan::new(
                                    *comp
                                        .port_attrs
                                        .get(output)
                                        .get(utils::PortFloat::CombDelay)
                                        .unwrap_or_else(
                                            || panic!("Combinational delay not found for port {}", comp.display(output))
                                        )
                                )
                                .expect(
                                    "Combinational delays should not be NaN",
                                ),
                            )
                        })
                        .collect_vec();

                    for src in inputs {
                        for (dst, delay) in &outputs {
                            edges.get_mut(src).push(*dst, *delay);
                        }
                    }
                }
                ir::Command::Connect(ir::Connect { src, dst, .. }) => {
                    assert!(src.is_port(comp) && dst.is_port(comp), "Bundles should be resolved before constructing dataflow");

                    edges
                        .get_mut(src.port)
                        .push(dst.port, NotNan::new(0.0).unwrap());
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

        Self { comp, edges }
    }
}

impl CombDataflow<'_> {
    fn critical_paths_rec(
        &self,
        path: LinkedHashSet<ir::PortIdx>,
        len: f64,
    ) -> Vec<Vec<ir::PortIdx>> {
        let current = path.iter().last().copied().unwrap();

        let mut chain = if len >= 1.0 {
            // This path is critical
            vec![path.iter().cloned().collect()]
        } else {
            vec![]
        };

        for (succ, &delay) in self.edges.get(current).iter() {
            let delay: f64 = delay.into();
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
        for src in 0..self.comp.ports().len() {
            let src = ir::PortIdx::new(src);
            let mut path = LinkedHashSet::new();

            // Find the combinational delay of this port
            let comb_delay = self
                .comp
                .port_attrs
                .get(src)
                .get(utils::PortFloat::CombDelay)
                .copied()
                .unwrap_or_default();

            path.insert(src);
            paths.extend(self.critical_paths_rec(path, comb_delay).into_iter());
        }

        paths
    }
}
