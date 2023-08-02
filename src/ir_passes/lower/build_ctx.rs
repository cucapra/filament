use super::utils::{comp_name, expr_u64, interface_name, port_name};
use super::Fsm;
use crate::ir::DenseIndexInfo;
use crate::ir::{self, Ctx};
use crate::ir_passes::lower::utils::INTERFACE_PORTS;
use calyx::structure;
use calyx_ir::{self as calyx, RRC};
use itertools::Itertools;
use std::{collections::HashMap, iter, rc::Rc};

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
    lib: &'a calyx::LibrarySignatures,
    fsms: HashMap<ir::EventIdx, Fsm>,
    /// Mapping from [ir::InstIdx]s to the calyx cell instantiated.
    instances: DenseIndexInfo<ir::Instance, RRC<calyx::Cell>>,
    /// Mapping from [ir::InstIdx]s to a reference of the calyx cell instantiated/invoked
    invokes: DenseIndexInfo<ir::Invoke, RRC<calyx::Cell>>,
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
            comp: ctx.get(idx),
            binding,
            builder,
            lib,
            instances: DenseIndexInfo::default(),
            invokes: DenseIndexInfo::default(),
            fsms: HashMap::new(),
        }
    }

    /// Adds an instance to the component
    pub fn add_instance(&mut self, idx: ir::InstIdx) {
        let inst = self.comp.get(idx);
        // generate a unique name for this instance
        let inst_name = format!("inst{}", idx.get());
        let comp_name = comp_name(inst.comp, self.ctx);

        let cell = if let Some(sig) = self.binding.get(&inst.comp) {
            // this component has is in the binding signature (it has been compiled and is non-primitive)
            self.builder.add_component(
                inst_name, // non-primitive component
                comp_name, sig,
            )
        } else {
            // this instance must be referring to a primitive, so we add one to the component

            // gets the parameters of this instance as concrete numbers
            let conc_bind = inst
                .params
                .iter()
                .map(|v| expr_u64(*v, self.comp))
                .collect_vec();
            self.builder.add_primitive(inst_name, comp_name, &conc_bind)
        };

        cell.borrow_mut()
            .attributes
            .insert(calyx::BoolAttr::Data, 1);

        // add this instance to the instance mapping
        self.instances.push(idx, cell);
    }

    /// Adds an invocation to the component
    pub fn add_invoke(&mut self, invidx: ir::InvIdx) {
        let inv = self.comp.get(invidx);

        // Gets a reference to the instance being invoked
        let cell = &self.instances[inv.inst];

        // loop through the event bindings defined in the instance and connect them to the corresponding fsms.
        for eb in inv.events.iter() {
            // If there is no interface port, no binding necessary
            if let Some(dst) = eb.base.apply(interface_name, self.ctx) {
                let ir::EventBind { arg: time, .. } = eb;

                // gets the interface port from the signature of the instance
                let dst = cell.borrow().get(dst);

                let time = self.comp.get(*time);
                let offset = expr_u64(time.offset, self.comp);
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

        // add a copy of the instance pointer to the invoke mapping
        self.invokes.push(invidx, Rc::clone(cell));
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
            "Range `{}` cannot be represented as a simple offset",
            self.comp.display_range(range)
        );

        let ev = start.event;

        // Don't generate a guard if there is no interface port
        if !self.comp.get(ev).has_interface {
            return calyx::Guard::True;
        }

        // return a guard that is active whenever from for all states from `start..end`
        let fsm = self.fsms.get(&ev).unwrap();
        (expr_u64(start.offset, self.comp)..expr_u64(end.offset, self.comp))
            .map(|st| fsm.get_port(st).into())
            .reduce(calyx::Guard::or)
            .unwrap()
    }

    /// Compiles an [ir::Port], returning the proper guard if present.
    pub fn compile_port(
        &mut self,
        idx: ir::PortIdx,
    ) -> (RRC<calyx::Port>, calyx::Guard<calyx::Nothing>) {
        let port = self.comp.get(idx);

        let name = port_name(idx, self.ctx, self.comp);

        let guard = self.compile_range(&port.live.range);
        let cell = match port.owner {
            ir::PortOwner::Sig { .. } => {
                self.builder.component.signature.borrow()
            }
            ir::PortOwner::Inv { inv, .. } => self.invokes[inv].borrow(),
            ir::PortOwner::Local => {
                unreachable!("Local ports should have been eliminated.")
            }
        };
        (cell.get(name), guard)
    }

    /// Compiles an [ir::Connect] by building the port assignments in calyx
    pub fn compile_connect(&mut self, con: &ir::Connect) {
        let ir::Connect { dst, src, .. } = con;

        assert!(
            expr_u64(src.start, self.comp) == 0
                && expr_u64(src.end, self.comp) == 1,
            "Port bundles should have been compiled away."
        );

        assert!(
            expr_u64(dst.start, self.comp) == 0
                && expr_u64(dst.end, self.comp) == 1,
            "Port bundles should have been compiled away."
        );

        log::debug!(
            "Compiling connect: {}",
            ir::Printer::new(self.comp).connect_str(con)
        );

        // ignores the guard of the destination (bind check already verifies that it is available for at least as long as src)
        let (dst, _) = self.compile_port(dst.port);
        let (src, g) = self.compile_port(src.port);
        debug_assert!(
            src.borrow().width == dst.borrow().width,
            "Invalid assignment. `{}.{}' has width {} and `{}.{}' has width {}",
            src.borrow().get_parent_name(),
            src.borrow().name,
            src.borrow().width,
            dst.borrow().get_parent_name(),
            dst.borrow().name,
            dst.borrow().width,
        );
        let assign = self.builder.build_assignment(dst, src, g);
        self.builder.component.continuous_assignments.push(assign);
    }

    /// Attempts to declare an fsm component (if not already declared) in the [Binding] stored by this [BuildCtx]
    /// and creates an [Fsm] from this [calyx::Component] FSM and stores it in the [BuildCtx]
    pub fn insert_fsm(&mut self, event: ir::EventIdx, states: u64) {
        // Construct an fsm iff the event is connected to an interface port
        if self.comp.get(event).has_interface {
            self.declare_fsm(states);

            // Construct the FSM
            let fsm = Fsm::new(event, states, self);
            self.fsms.insert(event, fsm);
        }
    }

    /// Creates a [calyx::Component] representing an FSM with a certain number of states to bind to each event.
    /// Adds component to the [Binding] held by this component.
    fn declare_fsm(&mut self, states: u64) {
        // If this fsm is already declared, just return.
        if self.binding.fsm_comps.contains_key(&states) {
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
