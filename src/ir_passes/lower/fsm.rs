use std::{collections::HashMap, iter, ops::Not};

use super::{utils::interface_name, Binding, BuildCtx};
use crate::{ir, ir_passes::lower::utils::INTERFACE_PORTS};
use calyx::{structure, Guard, Nothing};
use calyx_ir::{self as calyx, RRC};
use itertools::Itertools;

#[derive(Default)]
/// Represents an fsm component.
pub(super) struct FsmBind {
    /// Simple fsm components that have been created.
    simple: HashMap<u64, calyx::Component>,
    /// Counter fsms that have been created.
    counter: HashMap<u64, calyx::Component>,
}

impl FsmBind {
    /// Helper function that generates a [calyx:Component] for a counter with `n` states.
    fn add_counter(&mut self, states: u64, lib: &calyx::LibrarySignatures) {
        if self.counter.contains_key(&states) {
            return;
        }

        // gets the number of bits needed to represent the counter state.
        let bitwidth = (64 - (states - 1).leading_zeros()) as u64;

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
            .chain(INTERFACE_PORTS.iter().map(|(attr, pd)| calyx::PortDef {
                name: pd.0.into(),
                width: pd.1,
                direction: pd.2.clone(),
                attributes: vec![*attr].try_into().unwrap(),
            }))
            .chain(iter::once(
                (calyx::Id::from("go"), 1, calyx::Direction::Input).into(),
            ))
            .collect();

        let mut comp = calyx::Component::new(
            calyx::Id::from(format!("counter_{}", states)),
            ports,
            false,
            false,
            None,
        );

        comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);
        let mut builder = calyx::Builder::new(&mut comp, lib).not_generated();

        let state = builder.add_primitive("state", "std_reg", &[bitwidth]);

        let add = builder.add_primitive("add", "std_add", &[bitwidth]);

        // Constant signal
        structure!(builder;
            let signal_on = constant(1, 1);
            let zero = constant(0, bitwidth);
            let one = constant(1, bitwidth);
        );

        let zero = zero.borrow();
        let one = one.borrow();

        // generates the equality checks for each state output.
        let state_guards = (0..states)
            .map(|i| {
                let val = builder.add_constant(i, bitwidth);

                let eq = builder.add_primitive("done", "std_eq", &[bitwidth]);

                // counter is done when state == end.
                builder.build_assignment(
                    eq.borrow().get("left"),
                    state.borrow().get("out"),
                    Guard::<Nothing>::True,
                );

                builder.build_assignment(
                    eq.borrow().get("right"),
                    val.borrow().get("out"),
                    Guard::<Nothing>::True,
                );
                (i, eq)
            })
            .collect_vec();

        // This component's interface
        let this = builder.component.signature.borrow();
        // add base assignments
        builder.component.continuous_assignments.extend([
            // build assignments for state+1
            builder.build_assignment(
                add.borrow().get("left"),
                state.borrow().get("out"),
                Guard::<Nothing>::True,
            ),
            builder.build_assignment(
                add.borrow().get("right"),
                one.get("out"),
                Guard::<Nothing>::True,
            ),
            // hook up adder to register
            builder.build_assignment(
                state.borrow().get("in"),
                add.borrow().get("out"),
                Guard::<Nothing>::port(this.get(format!("_{}", states - 1)))
                    .not(),
            ),
            builder.build_assignment(
                state.borrow().get("in"),
                zero.get("out"),
                Guard::<Nothing>::port(this.get(format!("_{}", states - 1))),
            ),
            // increment always
            builder.build_assignment(
                state.borrow().get("write_en"),
                signal_on.borrow().get("out"),
                Guard::<Nothing>::True,
            ),
        ]);

        for (i, eq) in state_guards {
            builder.component.continuous_assignments.push(
                builder.build_assignment(
                    this.get(format!("_{}", i)),
                    eq.borrow().get("out"),
                    Guard::<Nothing>::True,
                ),
            );
        }

        drop(this);
        self.counter.insert(states, comp);
    }

    /// Helper function that generates a [calyx::Component] for an fsm with `n` states.
    pub fn add_simple(&mut self, states: u64, lib: &calyx::LibrarySignatures) {
        if self.simple.contains_key(&states) {
            return;
        }

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
            Guard::True,
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
                    Guard::True,
                )
            } else {
                let prev_cell = regs[(idx - 1) as usize].borrow();
                builder.build_assignment(
                    cell.get("in"),
                    prev_cell.get("out"),
                    Guard::True,
                )
            };
            let enable = builder.build_assignment(
                cell.get("write_en"),
                signal_on.borrow().get("out"),
                Guard::True,
            );
            let out = builder.build_assignment(
                this.get(format!("_{}", idx + 1)),
                cell.get("out"),
                Guard::True,
            );
            builder.component.continuous_assignments.extend([
                write_assign,
                enable,
                out,
            ]);
        }

        drop(this);
        self.simple.insert(states, comp);
    }

    pub fn get(&self, states: u64) -> &calyx::Component {
        self.simple.get(&states).unwrap()
    }

    pub fn take(self) -> Vec<calyx::Component> {
        self.simple
            .into_values()
            .chain(self.counter.into_values())
            .collect()
    }
}

/// A wrapper for a [calyx::Component] representing a finite state machine instance.
pub(super) struct Fsm {
    /// The [calyx::Component] representing this fsm.
    cell: RRC<calyx::Cell>,
}

impl Fsm {
    // Create a new [Fsm] for an [ir::EventIdx] with the given number of `states`.
    pub fn new(event: ir::EventIdx, states: u64, ctx: &mut BuildCtx) -> Self {
        let comp = ctx.binding.fsm_comps.get(states);

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
            Guard::True,
        );
        ctx.builder.component.continuous_assignments.push(go_assign);
        Fsm { cell }
    }

    /// Returns the [calyx::Port] that represents the given `state`.
    pub fn get_port(&self, state: u64) -> RRC<calyx::Port> {
        self.cell.borrow().get(format!("_{}", state))
    }
}
