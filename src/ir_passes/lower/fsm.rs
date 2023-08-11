use std::iter;

use super::{utils::interface_name, Binding, BuildCtx};
use crate::{ir, ir_passes::lower::utils::INTERFACE_PORTS};
use calyx::structure;
use calyx_ir::{self as calyx, RRC};
use itertools::Itertools;

/// Represents an fsm component.
pub(super) struct FsmComp {}

/// A wrapper for a [calyx::Component] representing a finite state machine instance.
pub(super) struct Fsm {
    /// The [calyx::Component] representing this fsm.
    cell: RRC<calyx::Cell>,
}

impl Fsm {
    // Create a new [Fsm] for an [ir::EventIdx] with the given number of `states`.
    pub fn new(event: ir::EventIdx, states: u64, ctx: &mut BuildCtx) -> Self {
        let comp = ctx.binding.fsm_comps.get(&states).unwrap();

        let Some(name) = interface_name(event, ctx.comp) else {
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

    /// Helper function that generates a [calyx::Component] for an fsm with `n` states.
    pub fn simple(
        states: u64,
        lib: &calyx::LibrarySignatures,
    ) -> calyx::Component {
        let ports: Vec<calyx::PortDef<u64>> = (0..states)
            // create the state ports in the format `_state`.
            .map(|n| {
                (
                    calyx::Id::from(format!("_{n}")),
                    1,
                    calyx::Direction::Output,
                )
                    .into()
            })
            // adds the `clk` and `reset` ports to the interface
            .chain(INTERFACE_PORTS.iter().map(|(attr, pd)| calyx::PortDef {
                name: pd.0.into(),
                width: pd.1,
                direction: pd.2.clone(),
                attributes: vec![*attr].try_into().unwrap(),
            }))
            // adds a `go` port
            .chain(iter::once(
                (calyx::Id::from("go"), 1, calyx::Direction::Input).into(),
            ))
            .collect();

        let mut comp = calyx::Component::new(
            calyx::Id::from(format!("fsm_{}", states)),
            ports,
            false,
            false,
            None,
        );

        comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);
        let mut builder = calyx::Builder::new(&mut comp, lib).not_generated();

        // Add n-1 registers
        let regs = (0..states - 1)
            .map(|_| builder.add_primitive("r", "std_reg", &[1]))
            .collect_vec();

        // Constant signal
        structure!(builder;
            let signal_on = constant(1, 1);
        );
        // This component's interface
        let this = builder.component.signature.borrow();

        // _0 = go;
        let assign = builder.build_assignment(
            this.get("_0"),
            this.get("go"),
            calyx::Guard::True,
        );
        builder.component.continuous_assignments.push(assign);

        // For each register, add the following assignments:
        // rn.write_en = 1'd1;
        // rn.in = r{n-1}.out;
        // _n = rn.out;
        for idx in 0..states - 1 {
            let cell = regs[idx as usize].borrow();
            let write_assign = if idx == 0 {
                builder.build_assignment(
                    cell.get("in"),
                    this.get("go"),
                    calyx::Guard::True,
                )
            } else {
                let prev_cell = regs[(idx - 1) as usize].borrow();
                builder.build_assignment(
                    cell.get("in"),
                    prev_cell.get("out"),
                    calyx::Guard::True,
                )
            };
            let enable = builder.build_assignment(
                cell.get("write_en"),
                signal_on.borrow().get("out"),
                calyx::Guard::True,
            );
            let out = builder.build_assignment(
                this.get(format!("_{}", idx + 1)),
                cell.get("out"),
                calyx::Guard::True,
            );
            builder.component.continuous_assignments.extend([
                write_assign,
                enable,
                out,
            ]);
        }
        drop(this);
        comp
    }
}
