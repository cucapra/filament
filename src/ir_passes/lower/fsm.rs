use std::{collections::HashMap, iter, ops::Not};

use super::{
    utils::{cell_to_port_def, interface_name},
    BuildCtx,
};
use crate::{ir, ir_passes::lower::utils::INTERFACE_PORTS};
use calyx::{build_assignments, guard, structure, Guard, Nothing};
use calyx_ir::{self as calyx, RRC};
use itertools::Itertools;

#[derive(Hash, PartialEq, Eq, Debug)]
/// Enum representing the types of fsms that can be generated and their indexing.
enum FsmType {
    /// A simple fsm with `n` states.
    Simple(u64),
    /// A counter fsm with `n` states.
    Counter(u64),
    /// A counter chain fsm with `n` counters each with `d` states.
    CounterChain(u64, u64),
}

#[derive(Default)]
/// Represents an fsm component.
pub(super) struct FsmBind {
    /// Fsm components using `n` counters each with `d` states, stored under `(n, d)`.
    fsms: HashMap<FsmType, calyx::Component>,
}

impl From<(u64, u64)> for FsmType {
    /// Generates an FsmType based on the default heuristic on the number of states and the delay (II).
    fn from((states, delay): (u64, u64)) -> Self {
        // TODO: Find a better metric to decide which type of fsm to generate.
        if delay > 1 {
            FsmType::CounterChain(states, delay)
        } else {
            FsmType::Simple(states)
        }
    }
}

impl FsmBind {
    /// Get an fsm with the number of states and minimum delay (II) from the binding
    pub fn get(&mut self, states: u64, delay: u64) -> &calyx::Component {
        self.add_opt(states, delay, None)
    }

    /// Insert an fsm with a number of states and a delay (II) into the binding
    pub fn add(
        &mut self,
        states: u64,
        delay: u64,
        lib: &calyx::LibrarySignatures,
    ) -> &calyx::Component {
        self.add_opt(states, delay, Some(lib))
    }

    fn add_opt(
        &mut self,
        states: u64,
        delay: u64,
        lib: Option<&calyx::LibrarySignatures>,
    ) -> &calyx::Component {
        match (states, delay).into() {
            FsmType::Simple(states) => self.add_simple(states, lib),
            FsmType::Counter(states) => self.add_counter(states, lib),
            FsmType::CounterChain(states, delay) => {
                let fsm_num = states / delay + (states % delay != 0) as u64;
                self.add_counter_chain(fsm_num, delay, lib)
            }
        }
    }

    /// Takes all the generated calyx::Components and returns them.
    pub fn take(self) -> Vec<calyx::Component> {
        self.fsms.into_values().collect()
    }

    /// Helper function that generates a [calyx::Component] for an fsm chaining `states` counters with `delay` states each.
    fn add_counter_chain(
        &mut self,
        fsm_num: u64,
        delay: u64,
        lib: Option<&calyx::LibrarySignatures>,
    ) -> &calyx::Component {
        let counter = self.add_counter(delay, lib);
        let (name, sig) = (
            counter.name.to_string(),
            cell_to_port_def(&counter.signature),
        );

        self.fsms
            .entry(FsmType::CounterChain(fsm_num, delay))
            .or_insert_with(|| {
                // gets the number of bits needed to represent the counter state.
                let bitwidth = (64 - (delay - 1).leading_zeros()) as u64;

                let ports: Vec<calyx::PortDef<u64>> = (0..fsm_num)
                    // create the state ports in the format `_state`.
                    .flat_map(|n| {
                        [
                            (
                                calyx::Id::from(format!("_{n}state")),
                                bitwidth,
                                calyx::Direction::Output,
                            )
                                .into(),
                            (
                                calyx::Id::from(format!("_{n}_0")),
                                1,
                                calyx::Direction::Output,
                            )
                                .into(),
                        ]
                    })
                    .chain(INTERFACE_PORTS.iter().map(|(attr, pd)| {
                        calyx::PortDef {
                            name: pd.0.into(),
                            width: pd.1,
                            direction: pd.2.clone(),
                            attributes: vec![*attr].try_into().unwrap(),
                        }
                    }))
                    .chain([
                        (calyx::Id::from("go"), 1, calyx::Direction::Input)
                            .into(),
                        (calyx::Id::from("done"), 1, calyx::Direction::Output)
                            .into(),
                    ])
                    .collect();

                let mut comp = calyx::Component::new(
                    calyx::Id::from(format!(
                        "counter_chain_{}_{}",
                        fsm_num, delay
                    )),
                    ports,
                    false,
                    false,
                    None,
                );

                comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);

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
                        counters[(fsm - 1) as usize].borrow().get("done")
                    };

                    // hook up the end of the last fsm to this one's start.
                    builder.component.continuous_assignments.extend([
                        builder.build_assignment(c.get("go"), go, Guard::True),
                        builder.build_assignment(
                            this.get(format!("_{}state", fsm)),
                            c.get("state"),
                            Guard::True,
                        ),
                        builder.build_assignment(
                            this.get(format!("_{}_0", fsm)),
                            c.get("_0"),
                            Guard::True,
                        ),
                    ]);
                }

                // done <= _{n-1};
                // unused at the moment but useful if we want to chain FSMs.
                builder.component.continuous_assignments.push(
                    builder.build_assignment(
                        this.get("done"),
                        counters[(fsm_num - 1) as usize].borrow().get("done"),
                        Guard::True,
                    ),
                );

                drop(this);
                comp
            })
    }

    /// Helper function that generates a [calyx:Component] for a counter with `n` states.
    fn add_counter(
        &mut self,
        states: u64,
        lib: Option<&calyx::LibrarySignatures>,
    ) -> &calyx::Component {
        self.fsms
            .entry(FsmType::Counter(states))
            .or_insert_with(|| {
                // gets the number of bits needed to represent the counter state.
                let bitwidth = (64 - (states - 1).leading_zeros()) as u64;

                let ports: Vec<calyx::PortDef<u64>> = INTERFACE_PORTS
                    .iter()
                    .map(|(attr, pd)| calyx::PortDef {
                        name: pd.0.into(),
                        width: pd.1,
                        direction: pd.2.clone(),
                        attributes: vec![*attr].try_into().unwrap(),
                    })
                    .chain([
                        (
                            calyx::Id::from("state"),
                            bitwidth,
                            calyx::Direction::Output,
                        )
                            .into(),
                        // need this port here because the 0 state is equivalent to go && state == 0
                        (calyx::Id::from("_0"), 1, calyx::Direction::Output)
                            .into(),
                        (calyx::Id::from("go"), 1, calyx::Direction::Input)
                            .into(),
                        (calyx::Id::from("done"), 1, calyx::Direction::Output)
                            .into(),
                    ])
                    .collect();

                let mut comp = calyx::Component::new(
                    calyx::Id::from(format!("counter_{}", states)),
                    ports,
                    false,
                    false,
                    None,
                );

                comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);

                let mut builder = calyx::Builder::new(&mut comp, lib.unwrap())
                    .not_generated();

                // Constant signal
                structure!(builder;
                    let signal_on = constant(1, 1);
                    let one = constant(1, bitwidth);
                    let zero = constant(0, bitwidth);
                    let add = prim std_add(bitwidth);
                    let state = prim std_reg(bitwidth);
                    let done = prim std_reg(1);
                    let final_state = constant(states-1, bitwidth);
                );

                // This component's interface
                let this = builder.component.signature.clone();

                // checks if the counter is currently on the final state.
                // state == _{n-1}
                let rst_check =
                    guard!(state["out"]).eq(guard!(final_state["out"]));

                // check if we should increment the counter.
                // go || state != 0
                let go_check = guard!(this["go"])
                    .or(guard!(state["out"]).eq(guard!(zero["out"])).not());

                let enable_check = rst_check.clone().or(go_check.clone());

                // go && state == 0
                let zero_check = guard!(this["go"])
                    .and(guard!(state["out"]).eq(guard!(zero["out"])));

                // add base assignments
                builder.component.continuous_assignments.extend(
                    build_assignments!(builder;
                        // build assignments for state+1
                        add["left"] = ? state["out"];
                        add["right"] = ? one["out"];
                        // hook up adder to register
                        state["in"] = go_check ? add["out"];
                        // hook up zero when counter finished
                        state["in"] = rst_check ? zero["out"];
                        // always enable register when there is an input
                        state["write_en"] = enable_check ? signal_on["out"];
                        // _0 = go && state == 0
                        this["_0"] = zero_check ? signal_on["out"];
                        // hook up state
                        this["state"] = ? state["out"];
                        // done <= _{n-1};
                        // unused at the moment but useful if we want to chain FSMs.
                        this["done"] = ? done["out"];
                        done["in"] = rst_check ? signal_on["out"];
                        done["write_en"] = ? signal_on["out"];
                    ),
                );

                drop(this);
                comp
            })
    }

    /// Helper function that generates a [calyx::Component] for an fsm with `n` states.
    fn add_simple(
        &mut self,
        states: u64,
        lib: Option<&calyx::LibrarySignatures>,
    ) -> &calyx::Component {
        self.fsms.entry(FsmType::Simple(states)).or_insert_with(|| {
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
                .chain(INTERFACE_PORTS.iter().map(|(attr, pd)| {
                    calyx::PortDef {
                        name: pd.0.into(),
                        width: pd.1,
                        direction: pd.2.clone(),
                        attributes: vec![*attr].try_into().unwrap(),
                    }
                }))
                .chain([
                    (calyx::Id::from("go"), 1, calyx::Direction::Input).into(),
                    (calyx::Id::from("done"), 1, calyx::Direction::Output)
                        .into(),
                ])
                .collect();

            let mut comp = calyx::Component::new(
                calyx::Id::from(format!("fsm_{}", states)),
                ports,
                false,
                false,
                None,
            );

            comp.attributes.insert(calyx::BoolAttr::NoInterface, 1);

            let mut builder =
                calyx::Builder::new(&mut comp, lib.unwrap()).not_generated();

            // Add n-1 registers
            let regs = (0..states)
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
            for idx in 0..states {
                let cell = regs[idx as usize].borrow();

                let prev_done = if idx == 0 {
                    this.get("go")
                } else {
                    let prev_cell = regs[(idx - 1) as usize].borrow();
                    prev_cell.get("out")
                };

                let out_port = if idx == states - 1 {
                    this.get("done")
                } else {
                    this.get(format!("_{}", idx + 1))
                };

                let write_assign = builder.build_assignment(
                    cell.get("in"),
                    prev_done,
                    Guard::True,
                );

                let enable = builder.build_assignment(
                    cell.get("write_en"),
                    signal_on.borrow().get("out"),
                    Guard::True,
                );

                let out = builder.build_assignment(
                    out_port,
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

    /// Generates a guard that is active for a range of states from start to end.
    pub(self) fn range_guard(
        builder: &mut calyx::Builder,
        cell: RRC<calyx::Cell>,
        ft: FsmType,
        // prefix here necessary for counter chain implementations to treat one fsm as a different type
        prefix: String,
        start: u64,
        end: u64,
    ) -> Guard<Nothing> {
        log::info!("{}, {}, {:?}", start, end, ft);

        match ft {
            FsmType::Simple(_) => (start..end)
                .map(|st| guard!(cell[format!("_{}", st)]))
                .reduce(calyx::Guard::or)
                .unwrap(),
            FsmType::Counter(states) => {
                let bitwidth = (64 - (states - 1).leading_zeros()) as u64;

                // if start is zero, we need to use its special port instead
                let (start, guard) = if start == 0 {
                    (start + 1, Some(guard!(cell[format!("{}_0", prefix)])))
                } else {
                    (start, None)
                };

                let start = builder.add_constant(start, bitwidth);
                // create a constant for end - 1 here to make the checks inclusive.
                // necessary if end is the final state and would require an extra bit to represent.
                let end = builder.add_constant(end - 1, bitwidth);

                // state >= start && state <= end
                let g = guard!(cell[format!("{}state", prefix)])
                    .ge(guard!(start["out"]))
                    .and(
                        guard!(cell[format!("{}state", prefix)])
                            .le(guard!(end["out"])),
                    );

                // generate the final guard
                guard.map_or(g.clone(), |gg| gg.or(g))
            }
            FsmType::CounterChain(_, delay) => {
                let fsm_start = start / delay;
                let fsm_end = (end - 1) / delay;

                if fsm_start == fsm_end {
                    // if the start and end are in the same fsm, we can treat it as a single counter
                    let start = start - fsm_start * delay;
                    let end = end - fsm_start * delay;

                    FsmBind::range_guard(
                        builder,
                        cell,
                        FsmType::Counter(delay),
                        format!("_{}", fsm_start),
                        start,
                        end,
                    )
                } else {
                    // end is in a different fsm from start
                    let start = start - fsm_start * delay;
                    let end = end - fsm_end * delay;

                    iter::once((start, delay))
                        .chain(
                            iter::repeat((0, delay))
                                .take((fsm_end - fsm_start - 1) as usize),
                        )
                        .chain(iter::once((0, end)))
                        .enumerate()
                        .map(|(i, (s, e))| {
                            FsmBind::range_guard(
                                builder,
                                cell.clone(),
                                FsmType::Counter(delay),
                                format!("_{}", i as u64 + fsm_start),
                                s,
                                e,
                            )
                        })
                        .reduce(calyx::Guard::or)
                        .unwrap()
                }
            }
        }
    }
}

/// A wrapper for a [calyx::Component] representing a finite state machine instance.
pub(super) struct Fsm {
    /// The [calyx::Component] representing this fsm.
    cell: RRC<calyx::Cell>,
    /// Number of states in this fsm.
    states: u64,
    /// II of this fsm.
    delay: u64,
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
        Fsm {
            cell,
            states,
            delay,
        }
    }

    /// Generates a guard that is active for a range of states from start to end.
    pub fn range_guard(
        &self,
        builder: &mut calyx::Builder,
        start: u64,
        end: u64,
    ) -> Guard<Nothing> {
        FsmBind::range_guard(
            builder,
            self.cell.clone(),
            (self.states, self.delay).into(),
            "".to_string(),
            start,
            end,
        )
    }
}
