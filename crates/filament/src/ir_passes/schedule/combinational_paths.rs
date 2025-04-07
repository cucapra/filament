use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils::{self as utils, AttrCtx};
use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use ordered_float::NotNan;
use std::collections::{HashMap, HashSet};

/// an index into a specific port in a bundle.
type BundleIdx = (ir::PortIdx, u64);

/// Combinational paths in a component
#[derive(Clone)]
pub struct CombDataflow {
    /// Combinational delays of the component
    delays: ir::DenseIndexInfo<ir::Port, NotNan<f64>>,
    /// Edges in the dataflow graph
    edges: HashMap<BundleIdx, HashSet<BundleIdx>>,
}

impl CombDataflow {
    pub fn new(comp: &ir::Component) -> Self {
        let mut edges: HashMap<BundleIdx, HashSet<BundleIdx>> = HashMap::new();
        let mut delays: ir::DenseIndexInfo<ir::Port, NotNan<f64>> =
            ir::DenseIndexInfo::with_default(comp.ports().len());

        for cmd in &comp.cmds {
            match cmd {
                ir::Command::Invoke(idx) => {
                    let inputs = idx.inputs(comp);
                    let outputs = idx.outputs(comp).collect_vec();

                    for &output in &outputs {
                        let delay = *comp
                            .port_attrs
                            .get(output)
                            .get(utils::PortFloat::CombDelay)
                            .unwrap_or_else(|| {
                                panic!(
                                    "Combinational delay not found for port {}",
                                    comp.display(output)
                                )
                            });

                        log::trace!(
                            "Combinational delay for {}: {}",
                            comp.display(output),
                            delay
                        );

                        delays.insert(
                            output,
                            NotNan::new(delay).expect(
                                "Combinational delays should not be NaN",
                            ),
                        );
                    }

                    // all outputs are dependent on all inputs
                    for src in inputs {
                        // Number of source bundle ports
                        let src_num = comp
                            .get(src)
                            .live
                            .lens
                            .iter()
                            .map(|&l| l.concrete(comp))
                            .product();

                        for &dst in &outputs {
                            let dst_num = comp
                                .get(dst)
                                .live
                                .lens
                                .iter()
                                .map(|&l| l.concrete(comp))
                                .product();

                            for src_idx in 0..src_num {
                                for dst_idx in 0..dst_num {
                                    edges
                                        .entry((src, src_idx))
                                        .or_default()
                                        .insert((dst, dst_idx));
                                }
                            }
                        }
                    }
                }
                ir::Command::Connect(ir::Connect { src, dst, .. }) => {
                    let src_lens = comp
                        .get(src.port)
                        .live
                        .lens
                        .iter()
                        .map(|l| l.concrete(comp))
                        .collect_vec();

                    let dst_lens = comp
                        .get(dst.port)
                        .live
                        .lens
                        .iter()
                        .map(|l| l.concrete(comp))
                        .collect_vec();

                    let src_idxs = utils::all_indices(
                        src.ranges
                            .iter()
                            .map(|(s, e)| (s.concrete(comp), e.concrete(comp)))
                            .collect(),
                    )
                    .into_iter()
                    .map(|idx| utils::flat_idx(&idx, &src_lens));

                    let dst_idxs = utils::all_indices(
                        dst.ranges
                            .iter()
                            .map(|(s, e)| (s.concrete(comp), e.concrete(comp)))
                            .collect(),
                    )
                    .into_iter()
                    .map(|idx| utils::flat_idx(&idx, &dst_lens));

                    // Zip the source and destination indices together
                    for (src_idx, dst_idx) in src_idxs.zip(dst_idxs) {
                        edges
                            .entry((src.port, src_idx))
                            .or_default()
                            .insert((dst.port, dst_idx));
                    }
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
        path: LinkedHashSet<BundleIdx>,
        len: f64,
    ) -> Vec<Vec<BundleIdx>> {
        let current = path.iter().last().copied().unwrap();

        if len >= 1.0 {
            // This path is critical here, we can return it immediately
            return vec![path.iter().cloned().collect()];
        }

        let mut chain = vec![];

        for &(succ, succ_idx) in
            self.edges.get(&current).unwrap_or(&HashSet::new()).iter()
        {
            let delay: f64 = (*self.delays.get(succ)).into();
            if path.contains(&(succ, succ_idx)) {
                unreachable!("Cycle detected in dataflow graph");
            }

            let mut new_path = path.clone();
            new_path.insert((succ, succ_idx));

            chain.extend(
                self.critical_paths_rec(new_path, len + delay).into_iter(),
            )
        }

        chain
    }
    /// Find the critical paths (sum of edges > clock period) in a DAG using Djikstra
    /// To do so we use DFS to find all paths from every source to every sink
    pub fn critical_paths(&self) -> Vec<Vec<BundleIdx>> {
        let mut paths = vec![];
        for (src, _) in self.edges.iter() {
            let mut path = LinkedHashSet::new();

            // Find the combinational delay of this port
            let comb_delay = *self.delays.get(src.0);

            path.insert(*src);
            paths.extend(
                self.critical_paths_rec(path, comb_delay.into()).into_iter(),
            );
        }

        paths
    }
}
