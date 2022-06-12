use super::Context;
use crate::errors::FilamentResult;
use crate::event_checker::ast;
use calyx::ir::{self, RRC};
use calyx::{build_assignments, guard, structure};
/// A Calyx FSM that increments every cycle.
pub struct Fsm {
    /// Fsm being constructed
    sig: ast::Fsm,

    /// Output port for the FSM register
    output_port: RRC<ir::Port>,

    /// Guard for the start event
    start_event: ir::Guard,
}

impl Fsm {
    /// Construct a new Fsm from signature. Instantiates assignments
    /// needed to start, increment and reset the fsm.
    pub fn new(sig: ast::Fsm, ctx: &mut Context) -> Self {
        let (trigger_port, guard) = ctx.compile_port(&sig.trigger);
        assert!(guard.is_none(), "Trigger port implies guard");
        let builder = &mut ctx.builder;

        // Construct circuitry for the FSM
        let fsm = builder.add_primitive(&*sig.name.id, "std_reg", &[32]);
        structure!(builder;
           let add = prim std_add(32);
           let zero = constant(0, 32);
           let one = constant(1, 32);
           let signal_on = constant(1, 1);
           let last = constant(sig.states, 32);
        );

        // go & fsm.out == 32'd0
        let fsm_out = guard!(fsm["out"]);
        let start =
            fsm_out.clone().eq(guard!(zero["out"])) & trigger_port.into();

        // (fsm.out > 0 & fsm.out < last) | start
        let incr = fsm_out.clone().gt(guard!(zero["out"]))
            & fsm_out.clone().lt(guard!(last["out"]))
            | start.clone();

        // fsm.out == last
        let end = fsm_out.eq(guard!(last["out"]));

        let mut assigns = build_assignments!(builder;
            // Increment assigns
            add["left"] = ? fsm["out"];
            add["right"] = ? one["out"];
            fsm["in"] = incr ? add["out"];
            fsm["write_en"] = incr ? signal_on["out"];

            // Reset assigns
            fsm["in"] = end ? zero["out"];
            fsm["write_en"] = end ? signal_on["out"];
        );

        builder
            .component
            .continuous_assignments
            .append(&mut assigns);

        let output_port = fsm.borrow().get("out");
        Fsm {
            sig,
            output_port,
            start_event: start,
        }
    }

    /// Generate guard associated with a particular state on the Fsm.
    pub fn event(
        &self,
        port: &ast::Id,
        builder: &mut ir::Builder,
    ) -> FilamentResult<ir::Guard> {
        let st = self.sig.state(port)?;
        if st == 0 {
            Ok(self.start_event.clone())
        } else {
            structure!(builder;
                let c = constant(st, 32);
            );
            let c_out = guard!(c["out"]);
            Ok(c_out.eq(self.output_port.clone().into()))
        }
    }
}
