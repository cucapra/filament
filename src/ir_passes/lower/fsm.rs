use crate::ir;
use calyx_ir::{self as calyx, RRC};

use super::{Binding, BuildCtx};

/// A wrapper for a [calyx::Component] representing a finite state machine.
pub(super) struct Fsm {
    /// The [calyx::Component] representing this fsm.
    cell: RRC<calyx::Cell>,
}

impl Fsm {
    // Create a new [Fsm] for an [ir::EventIdx] with the given number of `states`.
    pub fn new(event: ir::EventIdx, states: u64, ctx: &mut BuildCtx) -> Self {
        let comp = ctx.binding.fsm_comps.get(&states).unwrap();

        let Some(name) = event.interface_name(ctx.comp) else {
            unreachable!("Info should be an interface port");
        };

        // Creates a new component for the FSM.
        let cell = ctx.builder.add_component(
            name.clone(),
            comp.name.to_string(),
            Binding::cell_to_port_def(&comp.signature),
        );

        // gets the trigger port from the signature
        let sig = ctx.builder.component.signature.borrow();
        let trigger = sig.get(name);

        // Connect the trigger port to the instance
        let go_assign = ctx.builder.build_assignment(
            cell.borrow().get("go"),
            trigger,
            calyx::Guard::True,
        );
        ctx.builder.component.continuous_assignments.push(go_assign);
        Fsm { cell }
    }

    /// Returns the [calyx::Port] that represents the given `state`.
    pub fn get_port(&self, state: u64) -> RRC<calyx::Port> {
        self.cell.borrow().get(format!("_{}", state))
    }
}
