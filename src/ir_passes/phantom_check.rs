use crate::{
    cmdline,
    ir_visitor::{Action, Construct, Visitor, VisitorData},
};
use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils::{Diagnostics, Error, GPosIdx};
use itertools::Itertools;

/// Checks if a user-level phantom events are valid.
/// Phantom events are valid iff:
/// 1. The component doesn't share any instances
/// 2. The component doesn't use an subcomponents that need to use the
///    corresponding event in their interface, i.e., the uses of the event are all phantom
pub struct PhantomCheck {
    phantom_events: Vec<ir::EventIdx>,
    /// Instances defined in each scope
    defined_insts: Vec<Vec<ir::InstIdx>>,
    /// Mapping from instance to the first invoke seen
    diag: Diagnostics,
}

impl PhantomCheck {
    /// Check if an instance is defined in the current scope
    fn inst_def_in_scope(&self, inst: ir::InstIdx) -> bool {
        self.defined_insts.last().unwrap().contains(&inst)
    }

    /// Check if we're currently in a loop nest
    fn in_loop(&self) -> bool {
        self.defined_insts.len() != 1
    }
}

impl Construct for PhantomCheck {
    fn from(_: &cmdline::Opts, _: &mut ir::Context) -> Self {
        PhantomCheck {
            phantom_events: Vec::new(),
            defined_insts: vec![Vec::new()],
            diag: Diagnostics::default(),
        }
    }

    fn clear_data(&mut self) {
        self.phantom_events.clear();
        self.defined_insts = vec![Vec::new()];
        /* Diagnostics struct is shared */
    }
}

impl Visitor for PhantomCheck {
    fn name() -> &'static str {
        "phantom-check"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        let comp = &data.comp;
        self.phantom_events = comp.phantom_events().collect();
        if self.phantom_events.is_empty() {
            return Action::Stop;
        }

        let diag = &mut self.diag;

        // For each instance, check to see if any shared invocation uses a
        // phantom event.
        for (inst, invs) in comp.inst_invoke_map() {
            if invs.len() < 2 {
                continue;
            }

            let shared_inv = invs.iter().find_map(|inv| {
                let bind_loc = inv.times(comp).find_map(|(time, eb)| {
                    let ev = time.event(comp);
                    if self.phantom_events.contains(&ev) {
                        Some((
                            ev,
                            comp.get(eb)
                                .as_event_bind()
                                .map(|eb| eb.bind_loc)
                                .unwrap_or(GPosIdx::UNKNOWN),
                        ))
                    } else {
                        None
                    }
                });
                bind_loc.map(|b| (*inv, b))
            });

            if let Some((inv, (ev, bind_loc))) = shared_inv {
                let inst_info = comp.get(comp.get(inst).info).as_instance();
                let inst_bind = inst_info
                    .map(|info| info.bind_loc)
                    .unwrap_or(GPosIdx::UNKNOWN);
                let inv_info = comp.get(comp.get(inv).info).as_invoke();
                let inv_bind = inv_info
                    .map(|info| info.bind_loc)
                    .unwrap_or(GPosIdx::UNKNOWN);

                let err = Error::malformed(
                    "cannot reuse instance using a phantom event",
                )
                .add_note(diag.add_info(
                    format!("instance is invoked {} times", invs.len()),
                    inst_bind,
                ))
                .add_note(
                    diag.add_info("invocation uses phantom event", inv_bind),
                )
                .add_note(
                    diag.add_info(format!("event {} is a phantom event", comp.display(ev)), bind_loc)
                )
                .add_note(diag.add_message("phantom events are compiled away and cannot be used for resource sharing"));
                diag.add_error(err);
            }
        }

        Action::Continue
    }

    fn start_loop(&mut self, _: &mut ir::Loop, _: &mut VisitorData) -> Action {
        self.defined_insts.push(Vec::new());
        Action::Continue
    }

    fn end_loop(&mut self, _: &mut ir::Loop, _: &mut VisitorData) -> Action {
        self.defined_insts.pop();
        Action::Continue
    }

    fn instance(
        &mut self,
        inst: ir::InstIdx,
        _data: &mut VisitorData,
    ) -> Action {
        self.defined_insts.last_mut().unwrap().push(inst);
        Action::Continue
    }

    // For each invocation, ensure that:
    // 1. Invocations within loops do not use phantom events
    // 2. Phantom events are not provided when the underlying component requires non-phantom binding
    fn invoke(&mut self, inv: ir::InvIdx, data: &mut VisitorData) -> Action {
        // Check if the instance has already been used
        let comp = &data.comp;
        let ctx = data.ctx();
        let inst = inv.inst(comp);

        // If an invocation is within a loop, we need to ensure that its
        // corresponding instance is in the same loop nest.
        if self.in_loop() && !self.inst_def_in_scope(inst) {
            // If it is not, then ensure that there are no phantom events used
            if let Some(bind_loc) = inv.times(comp).find_map(|(time, eb)| {
                if self.phantom_events.contains(&time.event(comp)) {
                    Some(
                        comp.get(eb)
                            .as_event_bind()
                            .map(|eb| eb.bind_loc)
                            .unwrap_or(GPosIdx::UNKNOWN),
                    )
                } else {
                    None
                }
            }) {
                let inst_bind = comp
                    .get(comp.get(inst).info)
                    .as_instance()
                    .map(|i| i.bind_loc)
                    .unwrap_or(GPosIdx::UNKNOWN);
                let err =
                Error::malformed(
                    "invocation is within a loop but instance is not",
                )
                .add_note(
                    self.diag.add_info("invocation uses phantom event", bind_loc),
                )
                .add_note(self.diag.add_info(
                    "instance is not within the same loop",
                    inst_bind,
                ))
                .add_note(
                    self.diag.add_message("invocations within loops will be unrolled an imply instance sharing")
                );
                self.diag.add_error(err);
            }
        }

        // For each binding provided to a non-phantom port, check that the
        // mentioned events are not non-phantom
        // component being instantiated
        let inst_comp = ctx.get(inst.comp(comp));

        // Phantom events belonging to the component being instantiated
        // XXX(rachit): This is recomputed for every invocation which might get expensive.
        let inst_phantoms = inst_comp.phantom_events().collect_vec();
        for (event, (bind, info)) in
            inst_comp.events().idx_iter().zip(inv.times(comp))
        {
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !inst_phantoms.contains(&event) {
                let ev = &bind.event(comp);
                if self.phantom_events.contains(ev) {
                    let eb_info =
                        comp.get(info).as_event_bind().unwrap().bind_loc;
                    let phantom_info = comp
                        .get(comp.get(*ev).info)
                        .as_event()
                        .map(|ev| ev.bind_loc)
                        .unwrap_or(GPosIdx::UNKNOWN);
                    let inst_ev_info = inst_comp
                        .get(inst_comp.get(event).info)
                        .as_event()
                        .map(|ev| ev.bind_loc)
                        .unwrap_or(GPosIdx::UNKNOWN);

                    let err = Error::malformed("component provided phantom event binding to non-phantom event argument")
                    .add_note(self.diag.add_info("invoke provides phantom event", eb_info))
                    .add_note(self.diag.add_info("event is a phantom event", phantom_info))
                    .add_note(self.diag.add_info("instance's event is not phantom", inst_ev_info))
                    .add_note(self.diag.add_message("phantom ports are compiled away and cannot be used by subcomponents"));
                    self.diag.add_error(err);
                }
            }
        }
        Action::Continue
    }

    fn after_traversal(&mut self) -> Option<u64> {
        self.diag.report_all()
    }
}
