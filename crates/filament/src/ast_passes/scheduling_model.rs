use crate::ast_visitor::{Action, Construct, Visitor};
use fil_ast as ast;
use fil_utils::{self as utils, AttrCtx, Diagnostics, Error, GPosIdx};
use std::{collections::HashMap, fs};

type Model = HashMap<String, HashMap<String, f64>>;

/// Loads the scheduling model into attributes
pub struct SchedulingModel {
    /// Scheduling model mapping
    model: Model,
    /// Current component
    comp: Option<ast::Id>,
}

impl Construct for SchedulingModel {
    fn from(opts: &crate::cmdline::Opts, ast: &mut fil_ast::Namespace) -> Self {
        let model = opts
            .scheduling_model
            .as_ref()
            .map(|path| {
                toml::from_str(&fs::read_to_string(path).unwrap()).unwrap()
            })
            .unwrap_or_default();

        Self { model, comp: None }
    }

    fn clear_data(&mut self) {
        self.comp = None;
    }
}

impl Visitor for SchedulingModel {
    fn name() -> &'static str {
        "scheduling_model"
    }

    fn signature(&mut self, sig: &mut fil_ast::Signature) -> Action {
        self.comp = Some(*sig.name.inner());
        Action::Continue
    }
}
