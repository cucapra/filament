use crate::ast_visitor::{Action, Construct, Visitor};
use fil_ast as ast;
use fil_utils::{self as utils, AttrCtx, GPosIdx};
use serde::Deserialize;
use std::{collections::HashMap, fs};

#[derive(Deserialize, Debug, Default)]
struct CompModel {
    ports: HashMap<String, f64>,
    combinational: bool,
}

type Model = HashMap<String, CompModel>;

/// Loads the scheduling model into attributes
pub struct SchedulingModel {
    /// Scheduling model mapping
    model: Model,
}

impl Construct for SchedulingModel {
    fn from(opts: &crate::cmdline::Opts, _: &mut fil_ast::Namespace) -> Self {
        let model = opts
            .scheduling_model
            .as_ref()
            .map(|path| {
                toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
            })
            .unwrap_or_default();

        Self { model }
    }

    fn clear_data(&mut self) {}
}

impl Visitor for SchedulingModel {
    fn name() -> &'static str {
        "scheduling_model"
    }
    fn signature(&mut self, sig: &mut ast::Signature) -> Action {
        let comp = sig.name.inner().to_string();

        if let Some(attr_map) = self.model.get(&comp) {
            if attr_map.combinational {
                sig.attributes.set(
                    utils::CompBool::Combinational,
                    true,
                    GPosIdx::UNKNOWN,
                );
            }

            for port in sig.outputs_mut() {
                let name = port.name.to_string();

                let value = *attr_map.ports.get(&name).unwrap_or_else(|| panic!("Port {} not found in scheduling model for component {}",
                    name, comp));

                port.attrs.set(
                    utils::PortFloat::CombDelay,
                    value,
                    GPosIdx::UNKNOWN,
                );
            }
        }
        Action::Continue
    }
}
