use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils::{self as utils, AttrCtx};
use itertools::Itertools;

use crate::ir_visitor::{Action, Visitor, VisitorData};

/// Propogate combinational delays from foreign component to invocation ports
#[derive(Clone, Default)]
pub struct PropogateCombDelays;

impl Visitor for PropogateCombDelays {
    fn name() -> &'static str {
        "propogate-comb-delays"
    }

    fn invoke(
        &mut self,
        inv: fil_ir::InvIdx,
        data: &mut VisitorData,
    ) -> Action {
        let outputs = inv.outputs(&data.comp).collect_vec();

        for output in outputs {
            let ir::Port {
                owner: ir::PortOwner::Inv { base, .. },
                ..
            } = data.comp.get(output)
            else {
                unreachable!(
                    "Port {} was not an invocation port",
                    data.comp.display(output)
                );
            };

            // add the combinational delay to the output ports
            let (foreign_port, foreign_idx) = base.take();

            let foreign_comp = data.get(foreign_idx);

            if let Some(delay) = foreign_comp
                .port_attrs
                .get(foreign_port)
                .get(utils::PortFloat::CombDelay)
                .copied()
            {
                data.comp.port_attrs.get_mut(output).set(
                    utils::PortFloat::CombDelay,
                    delay,
                    utils::GPosIdx::UNKNOWN,
                );
            }
        }

        Action::Continue
    }
}
