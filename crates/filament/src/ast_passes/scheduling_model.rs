use crate::ast_visitor::{Action, Construct, Visitor};
use fil_utils::{self as utils, AttrCtx, GPosIdx};
use std::{collections::HashMap, fs};

type Model = HashMap<String, HashMap<String, f64>>;

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

    fn signature(&mut self, sig: &mut fil_ast::Signature) -> Action {
        let comp = sig.name.inner().to_string();

        if let Some(attr_map) = self.model.get(&comp) {
            for port in sig.ports.iter_mut() {
                let name = port.name.to_string();

                let value = *attr_map.get(&name).unwrap_or_else(|| panic!("Port {} not found in scheduling model for component {}",
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
