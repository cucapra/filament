use std::collections::HashMap;

use super::DenseIndexInfo;
use crate::{self as ir, Ctx};

/// Dataflow graph of a component
#[derive(Clone)]
pub struct Dataflow {
    preds: HashMap<ir::Port, Vec<ir::Port>>,
    succs: HashMap<ir::Port, Vec<ir::Port>>,
}

impl From<&ir::Component> for Dataflow {
    fn from(comp: &ir::Component) -> Self {
        let mut preds = DenseIndexInfo::with_default(comp.ports().len());
        let mut succs = DenseIndexInfo::with_default(comp.ports().len());

        for cmd in &comp.cmds {
            match cmd {
                crate::Command::Invoke(idx) => {
                    let inv = comp.get(*idx);
                    for port in &inv.ports {
                        for &pred in &port.pred {
                            preds.insert(*port, pred);
                        }
                    }
                }
                crate::Command::Instance(idx) => todo!(),
                crate::Command::BundleDef(idx) => todo!(),
                crate::Command::Connect(connect) => todo!(),
                crate::Command::Let(_) => todo!(),
                crate::Command::ForLoop(_) => todo!(),
                crate::Command::If(_) => todo!(),
                crate::Command::Fact(fact) => todo!(),
                crate::Command::Exists(exists) => todo!(),
            }
        }

        Self { preds, succs }
    }
}
