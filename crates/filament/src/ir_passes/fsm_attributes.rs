use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::TimeSub;
use fil_utils::{self as utils, AttrCtx};

/// Sets the proper FSM Attributes for every component
#[derive(Default)]
pub struct FSMAttributes;

impl Visitor for FSMAttributes {
    fn name() -> &'static str {
        "fsm-attributes"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        let attrs = &data.comp.attrs;

        // Check if the component already has FSM attributes
        if attrs.get(utils::BoolAttr::CounterFSM).is_some() {
            return Action::Stop;
        }

        // If the component is external or generated, do not add any slow FSM attributes
        if data.comp.is_ext() || data.comp.is_gen() {
            return Action::Stop;
        }

        // Get the delay of the component if it is a single event component
        if !data.opts.no_counter_fsms && data.comp.events().len() == 1 {
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
                data.comp.attrs.set(
                    utils::BoolAttr::CounterFSM,
                    true,
                    utils::GPosIdx::UNKNOWN,
                );
                return Action::Stop;
            }
        }

        data.comp.attrs.set(
            utils::BoolAttr::CounterFSM,
            false,
            utils::GPosIdx::UNKNOWN,
        );

        Action::Stop
    }
}
