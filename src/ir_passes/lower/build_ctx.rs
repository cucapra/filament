use std::{collections::HashMap, iter, rc::Rc};

use crate::{
    ir::{self, Ctx},
    utils,
};
use calyx::structure;
use calyx_ir::{self as calyx, RRC};
use itertools::Itertools;

use super::utils as compile_utils;
use super::Fsm;

/// Bindings associated with the current compilation context
#[derive(Default)]
pub(super) struct Binding {
    // Component signatures
    comps: HashMap<ir::CompIdx, RRC<calyx::Cell>>,
    /// Mapping to the component representing FSM with particular number of states
    pub fsm_comps: HashMap<u64, calyx::Component>,
}

impl Binding {
    /// Inserts a [calyx::Cell] into the binding
    pub fn insert(&mut self, name: ir::CompIdx, sig: RRC<calyx::Cell>) {
        self.comps.insert(name, sig);
    }

    /// Gets a [calyx::Cell]'s signature from an [ir::CompIdx]
    pub fn get(&self, idx: &ir::CompIdx) -> Option<Vec<calyx::PortDef<u64>>> {
        self.comps.get(idx).map(Self::cell_to_port_def)
    }

    /// Converts a cell to a list of port definitions
    pub fn cell_to_port_def(cr: &RRC<calyx::Cell>) -> Vec<calyx::PortDef<u64>> {
        let cell = cr.borrow();
        cell.ports()
            .iter()
            .map(|pr| {
                let port = pr.borrow();
                // Reverse port direction because signature refers to internal interface.
                (port.name, port.width, port.direction.reverse()).into()
            })
            .collect()
    }
}

/// Contains the context needed to compile a component.
pub(super) struct BuildCtx<'a> {
    pub builder: calyx::Builder<'a>,
    pub binding: &'a mut Binding,
    pub comp: &'a ir::Component,
    ctx: &'a ir::Context,
    cidx: ir::CompIdx,
    lib: &'a calyx::LibrarySignatures,
    fsms: HashMap<ir::EventIdx, Fsm>,
    /// Mapping from instances to cells
    instances: HashMap<ir::InstIdx, RRC<calyx::Cell>>,
    /// Mapping from invocation name to instance
    invokes: HashMap<ir::InvIdx, RRC<calyx::Cell>>,
}

impl<'a> BuildCtx<'a> {
    pub fn new(
        ctx: &'a ir::Context,
        idx: ir::CompIdx,
        binding: &'a mut Binding,
        builder: calyx::Builder<'a>,
        lib: &'a calyx::LibrarySignatures,
    ) -> Self {
        BuildCtx {
            ctx,
            cidx: idx,
            comp: ctx.get(idx),
            binding,
            builder,
            lib,
            instances: HashMap::new(),
            invokes: HashMap::new(),
            fsms: HashMap::new(),
        }
    }

    /// Adds an invocation to the component
    pub fn add_invoke(&mut self, invidx: ir::InvIdx) {
        let inv = self.comp.get(invidx);

        // Gets a reference to the instance being invoked
        let cell = &self
            .instances
            .get(&inv.inst)
            .unwrap_or_else(|| panic!("Unknown instance: {}", inv.inst));

        // loop through the event bindings defined in the instance and connect them to the corresponding fsms.
        for eb in inv.events.iter() {
            // If there is no interface port, no binding necessary
            if let Some(dst) = eb
                .base
                .apply(self.ctx, |comp, evt| evt.interface_name(comp))
            {
                let ir::EventBind { arg: time, .. } = eb;

                // gets the interface port from the signature of the instance
                let dst = cell.borrow().get(dst);

                let time = self.comp.get(*time);
                let offset = time.offset.as_u64(self.comp);
                // finds the corresponding port on the fsm of the referenced event
                let src = self.fsms.get(&time.event).unwrap().get_port(offset);

                let c = self.builder.add_constant(1, 1);

                // builds the assignment `dst = src ? 1'd1;`
                let assign = self.builder.build_assignment(
                    dst,
                    c.borrow().get("out"),
                    calyx::Guard::Port(src),
                );
                self.builder.component.continuous_assignments.push(assign);
            }
        }

        self.invokes.insert(invidx, Rc::clone(cell));
    }

    pub fn add_instance(&mut self, idx: ir::InstIdx) {
        let inst = self.comp.get(idx);
        let inst_name = self.idx_name(idx);
        let comp_name = inst.comp.name(self.ctx);

        let cell = if let Some(sig) = self.binding.get(&inst.comp) {
            self.builder.add_component(
                inst_name, // non-primitive component
                comp_name, sig,
            )
        } else {
            let conc_bind = inst
                .params
                .iter()
                .map(|v| v.as_u64(self.comp))
                .collect_vec();
            self.builder.add_primitive(inst_name, comp_name, &conc_bind)
        };

        cell.borrow_mut()
            .attributes
            .insert(calyx::BoolAttr::Data, 1);
        self.instances.insert(idx, cell);
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    /// Returns no guard if the related event has no interface port.
    pub fn compile_range(
        &mut self,
        range: &ir::Range,
    ) -> calyx::Guard<calyx::Nothing> {
        let start = self.comp.get(range.start);
        let end = self.comp.get(range.end);

        assert!(
            start.event == end.event,
            "Range `{range}` cannot be represented as a simple offset"
        );

        let ev = start.event;

        // return no guard the interface port does not exist.
        if self.comp.get(ev).has_interface {
            let fsm = self.fsms.get(&ev).unwrap();
            (start.offset.as_u64(self.comp)..end.offset.as_u64(self.comp))
                .map(|st| fsm.get_port(st).into())
                .reduce(calyx::Guard::or)
                .unwrap()
        } else {
            calyx::Guard::True
        }
    }

    pub fn compile_connect(&mut self, con: &ir::Connect) {
        let ir::Connect { dst, src, .. } = con;

        assert!(
            src.start.as_u64(self.comp) == 0 && src.end.as_u64(self.comp) == 1,
            "Port bundles should have been compiled away."
        );

        assert!(
            dst.start.as_u64(self.comp) == 0 && dst.end.as_u64(self.comp) == 1,
            "Port bundles should have been compiled away."
        );

        log::debug!("Compiling connect: {}", con);

        let (dst, _) = self.compile_port(dst.port);
        // assert!(!g.is_true(), "Destination port cannot have a guard.");
        let (src, g) = self.compile_port(src.port);
        let assign = self.builder.build_assignment(dst, src, g);
        self.builder.component.continuous_assignments.push(assign);
    }

    pub fn compile_port(
        &mut self,
        idx: ir::PortIdx,
    ) -> (RRC<calyx::Port>, calyx::Guard<calyx::Nothing>) {
        let port = self.comp.get(idx);

        let name = idx.name(self.ctx, self.comp);

        let guard = self.compile_range(&port.live.range);
        let cell = match port.owner {
            ir::PortOwner::Sig { .. } => {
                self.builder.component.signature.borrow()
            }
            ir::PortOwner::Inv { inv, .. } => self.invokes[&inv].borrow(),
            ir::PortOwner::Local => {
                unreachable!("Local ports should have been eliminated.")
            }
        };
        (cell.get(name), guard)
    }

    /// Attempts to declare an fsm (if not already declared) in the [Binding] stored by this [Context]
    /// and creates an [Fsm] from this [calyx::Component] FSM and stores it in the [Context]
    pub fn insert_fsm(&mut self, event: ir::EventIdx, states: u64) {
        // Construct an fsm iff the event is connected to an interface port
        if self.comp.get(event).has_interface {
            self.declare_fsm(states);

            // Construct the FSM
            let fsm = Fsm::new(event, states, self);
            self.fsms.insert(event, fsm);
        }
    }

    /// Creates a name from an [utils::Idx<T>] provided it has a prefix defined in the [utils::IdxPre] trait.
    fn idx_name<T>(&self, idx: utils::Idx<T>) -> String
    where
        ir::Component: Ctx<T>,
        utils::Idx<T>: utils::IdxPre,
    {
        format!(
            "{}_{}{}",
            self.cidx.name(self.ctx),
            <utils::Idx::<T> as utils::IdxPre>::prefix(),
            idx.get()
        )
    }

    /// Creates a [calyx::Component] representing an FSM with a certain number of states to bind to each event.
    /// Adds component to the [Binding] held by this component.
    fn declare_fsm(&mut self, states: u64) {
        if self.binding.fsm_comps.contains_key(&states) {
            return;
        }

        let ports: Vec<calyx::PortDef<u64>> = (0..states)
            .map(|n| {
                (
                    calyx::Id::from(format!("_{n}")),
                    1,
                    calyx::Direction::Output,
                )
                    .into()
            })
            .chain(compile_utils::INTERFACE_PORTS.iter().map(|(attr, pd)| {
                calyx::PortDef {
                    name: pd.0.into(),
                    width: pd.1,
                    direction: pd.2.clone(),
                    attributes: vec![*attr].try_into().unwrap(),
                }
            }))
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
        let mut builder =
            calyx::Builder::new(&mut comp, self.lib).not_generated();

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

        self.binding.fsm_comps.insert(states, comp);
    }
}
