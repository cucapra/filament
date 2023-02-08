use super::{Binding, Context};
use crate::event_checker::ast;
use calyx::ir::{self, RRC};
/// A Calyx FSM that increments every cycle.
pub struct Fsm {
    cell: RRC<ir::Cell>,
}

impl Fsm {
    // Construct a new FSM
    pub fn new(fsm: &ast::Fsm, ctx: &mut Context) -> Self {
        let event = &fsm.name;
        let comp = &ctx.binding.fsm_comps[&fsm.states];
        let cell = ctx.builder.add_component(
            format!("{event}"),
            comp.name.to_string(),
            Binding::cell_to_port_def(&comp.signature),
        );
        // Connect the trigger port to the instance
        let (port, guard) = ctx.compile_port(&fsm.trigger);
        assert!(guard.is_none(), "Trigger port implies guard");
        let go_assign = ctx.builder.build_assignment(
            cell.borrow().get("go"),
            port,
            ir::Guard::True,
        );
        ctx.builder.component.continuous_assignments.push(go_assign);
        Fsm { cell }
    }

    /// Generate guard associated with a particular state on the Fsm.
    pub fn event(&self, port: &ast::Id) -> ir::Guard {
        self.cell.borrow().get(format!("{port}")).into()
    }
}
