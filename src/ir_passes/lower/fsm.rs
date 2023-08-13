use std::{collections::HashMap, iter, ops::Not};

use super::{
    utils::{cell_to_port_def, interface_name},
    BuildCtx,
};
use crate::{ir, ir_passes::lower::utils::INTERFACE_PORTS};
use calyx::{structure, Guard, Nothing};
use calyx_ir::{self as calyx, RRC};
use itertools::Itertools;

#[derive(Default)]
/// Represents an fsm component.
pub(super) struct FsmBind {
    /// Simple fsm components that have been created.
    simple: HashMap<u64, calyx::Component>,
    /// Raw counter fsms that have been created
    counter: HashMap<u64, calyx::Component>,
    /// Fsm components using `n` counters each with `d` states, stored under `(n, d)`.
    counter_chain: HashMap<(u64, u64), calyx::Component>,
}

impl<'l> FsmBind {
    /// Get an fsm with the number of states and minimum delay (II) from the binding
    pub fn get(&'l mut self, states: u64, delay: u64) -> &'l calyx::Component {
        self.add_opt(states, delay, None)
    }

    /// Insert an fsm with a number of states and a delay (II) into the binding
    pub fn add(
        &'l mut self,
        states: u64,
        delay: u64,
        lib: &'l calyx::LibrarySignatures,
    ) -> &'l calyx::Component {
        self.add_opt(states, delay, Some(lib))
    }

    fn add_opt(
        &'l mut self,
        states: u64,
        delay: u64,
        lib: Option<&'l calyx::LibrarySignatures>,
    ) -> &'l calyx::Component {
        // TODO: Find a better metric to decide which type of fsm to generate.
        if delay > 1 {
            self.add_counter_chain(states, delay, lib)
        } else {
            self.add_simple(states, lib)
        }
    }

    /// Takes all the generated calyx::Components and returns them.
    pub fn take(self) -> Vec<calyx::Component> {
        self.simple
            .into_values()
            .chain(self.counter.into_values())
            .chain(self.counter_chain.into_values())
            .collect()
    }

    fn fsm_comp(pre: &str, states: u64) -> calyx::Component {
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
            calyx::Id::from(format!("{pre}_{}", states)),
            ports,
            false,
            false,
            None,
        );

        comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);
        comp
    }

    /// Helper function that generates a [calyx::Component] for an fsm chaining `states` counters with `delay` states each.
    fn add_counter_chain(
        &'l mut self,
        fsm_num: u64,
        delay: u64,
        lib: Option<&'l calyx::LibrarySignatures>,
    ) -> &'l calyx::Component {
        let counter = self.add_counter(delay, lib);
        let (name, sig) = (
            counter.name.to_string(),
            cell_to_port_def(&counter.signature),
        );

        self.counter_chain
            .entry((fsm_num, delay))
            .or_insert_with(|| {
                let mut comp =
                    FsmBind::fsm_comp("counter_chain", fsm_num * delay);
                let mut builder = calyx::Builder::new(&mut comp, lib.unwrap())
                    .not_generated();

                // all the counter components generated.
                let counters = (0..fsm_num)
                    .map(|fsm| {
                        builder.add_component(
                            format!("c{}", fsm),
                            name.clone(),
                            sig.clone(),
                        )
                    })
                    .collect_vec();

                // This component's interface
                let this = builder.component.signature.borrow();

                for fsm in 0..fsm_num {
                    let c = counters[fsm as usize].borrow();

                    let go = if fsm == 0 {
                        // this is the first fsm, start it when go is high
                        this.get("go")
                    } else {
                        // start this fsm when the previous one is done.
                        counters[(fsm - 1) as usize]
                            .borrow()
                            .get(format!("_{}", delay - 1))
                    };

                    // hook up the end of the last fsm to this one's start.
                    builder.component.continuous_assignments.push(
                        builder.build_assignment(
                            c.get("go"),
                            go,
                            calyx::Guard::<Nothing>::True,
                        ),
                    );

                    // build assignments to state output ports
                    for st in 0..delay {
                        builder.component.continuous_assignments.push(
                            builder.build_assignment(
                                this.get(format!("_{}", fsm * delay + st)),
                                c.get(format!("_{st}")),
                                calyx::Guard::<Nothing>::True,
                            ),
                        );
                    }
                }

                drop(this);
                comp
            })
    }

    /// Helper function that generates a [calyx:Component] for a counter with `n` states.
    fn add_counter(
        &'l mut self,
        states: u64,
        lib: Option<&'l calyx::LibrarySignatures>,
    ) -> &'l calyx::Component {
        self.counter.entry(states).or_insert_with(|| {
            // gets the number of bits needed to represent the counter state.
            let bitwidth = (64 - (states - 1).leading_zeros()) as u64;

            let mut comp = FsmBind::fsm_comp("counter", states);
            let mut builder =
                calyx::Builder::new(&mut comp, lib.unwrap()).not_generated();

            let state = builder.add_primitive("state", "std_reg", &[bitwidth]);

            let add = builder.add_primitive("add", "std_add", &[bitwidth]);

            // Constant signal
            structure!(builder;
                let signal_on = constant(1, 1);
                let one = constant(1, bitwidth);
                let zero = constant(0, bitwidth);
            );

            let one = one.borrow();

            // generates the equality checks for each state output.
            let consts = (0..states)
                .map(|i| (i, builder.add_constant(i, bitwidth)))
                .collect_vec();

            // This component's interface
            let this = builder.component.signature.borrow();

            // checks if the counter is currently on the final state.
            // state == _{n-1}
            let rst_check: Guard<Nothing> =
                Guard::Port(state.borrow().get("out")).eq(Guard::Port(
                    consts[(states - 1) as usize].1.borrow().get("out"),
                ));

            // check if we should increment the counter.
            // go || state != 0
            let go_check: Guard<Nothing> = Guard::Port(this.get("go")).or(
                Guard::Port(state.borrow().get("out"))
                    .eq(Guard::Port(zero.borrow().get("out")))
                    .not(),
            );

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
                    go_check.clone(),
                ),
                // hook up zero when counter finished
                builder.build_assignment(
                    state.borrow().get("in"),
                    zero.borrow().get("out"),
                    rst_check.clone(),
                ),
                // always enable register when there is an input
                builder.build_assignment(
                    state.borrow().get("write_en"),
                    signal_on.borrow().get("out"),
                    rst_check.or(go_check),
                ),
            ]);

            for (i, v) in consts {
                let guard: Guard<Nothing> =
                    Guard::Port(state.borrow().get("out"))
                        .eq(Guard::Port(v.borrow().get("out")));

                builder.component.continuous_assignments.push(
                    builder.build_assignment(
                        this.get(format!("_{}", i)),
                        signal_on.borrow().get("out"),
                        guard,
                    ),
                );
            }

            drop(this);
            comp
        })
    }

    /// Helper function that generates a [calyx::Component] for an fsm with `n` states.
    fn add_simple(
        &'l mut self,
        states: u64,
        lib: Option<&'l calyx::LibrarySignatures>,
    ) -> &'l calyx::Component {
        self.simple.entry(states).or_insert_with(|| {
            let mut comp = FsmBind::fsm_comp("fsm", states);
            let mut builder =
                calyx::Builder::new(&mut comp, lib.unwrap()).not_generated();

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
            comp
        })
    }
}

/// A wrapper for a [calyx::Component] representing a finite state machine instance.
pub(super) struct Fsm {
    /// The [calyx::Component] representing this fsm.
    cell: RRC<calyx::Cell>,
}

impl Fsm {
    // Create a new [Fsm] for an [ir::EventIdx] with the given number of `states`.
    pub fn new(
        event: ir::EventIdx,
        states: u64,
        delay: u64,
        ctx: &mut BuildCtx,
    ) -> Self {
        let comp = ctx.binding.fsm_comps.get(states, delay);

        let Some(name) = interface_name(event, ctx.comp) else {
            unreachable!("Info should be an interface port");
        };

        // Creates a new component for the FSM.
        let cell = ctx.builder.add_component(
            name.clone(),
            comp.name.to_string(),
            cell_to_port_def(&comp.signature),
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
