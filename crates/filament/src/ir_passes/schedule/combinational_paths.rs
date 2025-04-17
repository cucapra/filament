use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils::{self as utils, AttrCtx};
use itertools::Itertools;
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
                                log::warn!(
                                    "Combinational delay not found for port {}. Assuming zero.",
                                    comp.display(output)
                                );
                                &0.0
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

                    let inputs = inputs.into_iter().map(|src| {
                        (
                            src,
                            comp.get(src)
                                .live
                                .lens
                                .iter()
                                .map(|&l| l.concrete(comp))
                                .product(),
                        )
                    });

                    let outputs = outputs
                        .iter()
                        .map(|&dst| {
                            (
                                dst,
                                comp.get(dst)
                                    .live
                                    .lens
                                    .iter()
                                    .map(|&l| l.concrete(comp))
                                    .product(),
                            )
                        })
                        .collect_vec();

                    // all outputs are dependent on all inputs
                    for (src, src_num) in inputs {
                        for (dst, dst_num) in &outputs {
                            for src_idx in 0..src_num {
                                for dst_idx in 0..*dst_num {
                                    edges
                                        .entry((src, src_idx))
                                        .or_default()
                                        .insert((*dst, dst_idx));
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
                ir::Command::ForLoop(_) | ir::Command::If(_) => {
                    unreachable!("Components should be control-flow free")
                }
                ir::Command::BundleDef(_)
                | ir::Command::Instance(_)
                | ir::Command::Let(_)
                | ir::Command::Fact(_)
                | ir::Command::Exists(_) => {}
            }
        }

        Self { delays, edges }
    }

    fn critical_paths_rec(
        &self,
        current: BundleIdx,
        len: f64,
    ) -> Vec<BundleIdx> {
        if len >= 1.0 {
            // This path is critical here, we can return it immediately
            return vec![current];
        }

        let mut chain = vec![];

        for &(succ, succ_idx) in
            self.edges.get(&current).unwrap_or(&HashSet::new()).iter()
        {
            let delay: f64 = (*self.delays.get(succ)).into();

            chain.extend(self.critical_paths_rec((succ, succ_idx), len + delay))
        }

        chain
    }

    /// Find critical pairs in the DAG.
    pub fn critical_pairs(&self) -> Vec<(BundleIdx, BundleIdx)> {
        // First, get an indexing set of all the nodes in the graph
        let mut nodes = HashSet::new();
        for (src, dsts) in self.edges.iter() {
            nodes.insert(*src);
            for dst in dsts {
                nodes.insert(*dst);
            }
        }

        let nodes = nodes.into_iter().collect_vec();

        log::debug!("Calculating critical pairs for {} nodes", nodes.len());

        // Start by setting all max distances to edge values
        // Then use floyd-warshall to find the max distance between all pairs of nodes
        let mut max_distances = Vec::with_capacity(nodes.len());
        for (i, node) in nodes.iter().enumerate() {
            max_distances.push(vec![None; nodes.len()]);
            for (j, other_node) in nodes.iter().enumerate() {
                // Get the distance between the two nodes
                let dist: Option<f64> = self
                    .edges
                    .get(node)
                    .and_then(|dsts| dsts.get(other_node))
                    .map(|_| (*self.delays.get(other_node.0)).into());

                max_distances[i][j] = dist;
            }
        }

        // Floyd-Warshall algorithm
        for k in 0..nodes.len() {
            for i in 0..nodes.len() {
                for j in 0..nodes.len() {
                    if let (Some(d1), Some(d2)) =
                        (max_distances[i][k], max_distances[k][j])
                    {
                        if let Some(d) = max_distances[i][j] {
                            if d < d1 + d2 {
                                max_distances[i][j] = Some(d1 + d2);
                            }
                        } else {
                            max_distances[i][j] = Some(d1 + d2);
                        }
                    }
                }
            }
        }

        // Now we can find the critical pairs (all pairs that are greater than 1)
        let mut critical_pairs = vec![];
        for i in 0..nodes.len() {
            for j in 0..nodes.len() {
                if max_distances[i][j].is_some()
                    && max_distances[i][j].unwrap() > 1.0
                {
                    critical_pairs.push((nodes[i], nodes[j]));
                }
            }
        }

        critical_pairs
    }

    /// Find the critical paths (sum of edges > clock period) in a DAG using Djikstra
    /// To do so we use DFS to find all paths from every source to every sink
    pub fn critical_paths(&self) -> Vec<(BundleIdx, BundleIdx)> {
        let mut paths = vec![];
        for (src, _) in self.edges.iter() {
            // Find the combinational delay of this port
            let comb_delay = *self.delays.get(src.0);

            paths.extend(
                self.critical_paths_rec(*src, comb_delay.into())
                    .into_iter()
                    .map(|dst| (*src, dst)),
            );
        }

        paths
    }
}
