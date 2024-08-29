use crate::ir_visitor::{Visitor, VisitorData};
use fil_ast as ast;
use fil_ir::TimeSub;

/// Sets the proper FSM Attributes for every component
#[derive(Default)]
pub struct FSMAttributes;

impl Visitor for FSMAttributes {
    fn name() -> &'static str {
        "fsm-attributes"
    }

    fn visit(&mut self, mut data: VisitorData) {
        let attrs = &data.comp.attrs;

        // Check if the component already has FSM attributes
        if attrs.get_numeric(ast::NumAttr::SlowFSM).is_some() {
            return;
        }

        // If slow FSMs are disabled, do not add any slow FSM attributes
        if data.opts.disable_slow_fsms {
            return;
        }

        // If the component is external or generated, do not add any slow FSM attributes
        if data.comp.is_ext() || data.comp.is_gen() {
            return;
        }

        // Get the delay of the component if it is a single event component
        if data.comp.events().len() == 1 {
            let delay = &data.comp.events().iter().next().unwrap().1.delay;
            let TimeSub::Unit(delay) = delay else {
                data.comp.internal_error(
                    "Non-unit delays should have been compiled away.",
                );
            };
            let delay = delay.concrete(&data.comp);

            // If the delay is > 1, add a slow FSM attribute
            // TODO(UnsignedByte): Find a better heuristic for slow FSMs
            if delay > 1 {
                data.comp
                    .attrs
                    .set_numeric(ast::NumAttr::SlowFSM, Some(delay));
            }
        }
    }
}
