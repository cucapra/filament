use itertools::Itertools;

use super::DenseIndexInfo;
use crate as ir;

/// Dataflow graph of a component
#[derive(Clone)]
pub struct Dataflow {
    pub preds: DenseIndexInfo<ir::Port, Vec<ir::PortIdx>>,
    pub succs: DenseIndexInfo<ir::Port, Vec<ir::PortIdx>>,
}

impl From<&ir::Component> for Dataflow {
    fn from(comp: &ir::Component) -> Self {
        let mut preds: DenseIndexInfo<ir::Port, Vec<_>> =
            DenseIndexInfo::with_default(comp.ports().len());
        let mut succs: DenseIndexInfo<ir::Port, Vec<_>> =
            DenseIndexInfo::with_default(comp.ports().len());

        for cmd in &comp.cmds {
            match cmd {
                crate::Command::Invoke(idx) => {
                    let inputs = idx.inputs(comp);
                    let outputs = idx.outputs(comp).collect_vec();

                    for pred in inputs {
                        for succ in &outputs {
                            preds.get_mut(*succ).push(pred);
                            succs.get_mut(pred).push(*succ);
                        }
                    }
                }
                crate::Command::Connect(ir::Connect { src, dst, .. }) => {
                    assert!(src.is_port(comp) && dst.is_port(comp), "Bundles should be resolved before constructing dataflow");

                    preds.get_mut(dst.port).push(src.port);
                    succs.get_mut(src.port).push(dst.port);
                }
                crate::Command::BundleDef(_)
                | crate::Command::ForLoop(_)
                | crate::Command::If(_) => {
                    unreachable!(
                        "Components should be monomorphic and bundle-free"
                    )
                }
                crate::Command::Instance(_)
                | crate::Command::Let(_)
                | crate::Command::Fact(_)
                | crate::Command::Exists(_) => {}
            }
        }

        Self { preds, succs }
    }
}
