use itertools::Itertools;

use crate::{
    diagnostics,
    errors::Error,
    ir::{self, Ctx},
    ir_visitor::{Action, Visitor, VisitorData},
    utils::GPosIdx,
};
use std::collections::HashSet;

#[derive(Default)]
/// Checks if a user-level phantom events are valid.
/// Phantom events are valid iff:
/// 1. The component doesn't share any instances
/// 2. The component doesn't use an subcomponents that need to use the
///    corresponding event in their interface, i.e., the uses of the event are all phantom
pub struct PhantomCheck {
    instance_used: HashSet<ir::InstIdx>,
    prev_invoke: Option<ir::InvIdx>,
    phantom_events: Vec<ir::EventIdx>,
    diag: diagnostics::Diagnostics,
    diag_count: u32,
}

impl Visitor for PhantomCheck {
    fn name() -> &'static str {
        "phantom-check"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        let comp = &data.comp;
        self.phantom_events = comp.phantom_events().collect();
        if self.phantom_events.is_empty() {
            Action::Stop
        } else {
            Action::Continue
        }
    }

    fn invoke(&mut self, inv: ir::InvIdx, data: &mut VisitorData) -> Action {
        // Check if the instance has already been used
        let comp = &data.comp;
        let ctx = data.ctx();
        let inst = comp.get(inv).inst;
        if self.instance_used.get(&inst).is_some() {
            for (time, info) in inv.times(comp) {
                if let Some(e) =
                    self.phantom_events.iter().find(|e| time.event(comp) == **e)
                {
                    let bind_info =
                        comp.get(info).as_event_bind().unwrap().bind_loc;
                    let event_info = comp
                        .get(comp.get(*e).info)
                        .as_event()
                        .unwrap()
                        .bind_loc;
                    let prev_inv_info = comp
                        .get(comp.get(self.prev_invoke.unwrap()).info)
                        .as_invoke()
                        .unwrap()
                        .inst_loc;
                    let err =
                        Error::malformed("reuses instance uses phantom event for scheduling")
                    .add_note(self.diag.add_info("invocation uses phantom event", bind_info))
                    .add_note(self.diag.add_info("event is a phantom event", event_info))
                    .add_note(self.diag.add_info("previous use", prev_inv_info))
                    .add_note(self.diag.add_info("phantom ports are compiled away and cannot be used for resource sharing", GPosIdx::UNKNOWN));
                    self.diag.add_error(err);
                    self.diag_count += 1;
                    return Action::Stop;
                }
            }
        }
        self.instance_used.insert(inst);
        self.prev_invoke = Some(inv);

        // For each binding provided to a non-phantom port, check that the
        // mentioned events are not non-phantom

        // component being instantiated
        let inst_comp = ctx.get(comp.get(inst).comp);

        // phantom events belonging to the component being instantiated
        let inst_phantoms = inst_comp.phantom_events().collect_vec();
        for (event, (bind, info)) in
            inst_comp.events().idx_iter().zip(inv.times(comp).iter())
        {
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !inst_phantoms.contains(&event) {
                let ev = &bind.event(comp);
                if let Some(e) = self.phantom_events.iter().find(|e| *e == ev) {
                    let eb_info =
                        comp.get(*info).as_event_bind().unwrap().bind_loc;
                    let phantom_info = comp
                        .get(comp.get(*e).info)
                        .as_event()
                        .unwrap()
                        .bind_loc;
                    let inst_ev_info = inst_comp
                        .get(inst_comp.get(event).info)
                        .as_event()
                        .unwrap()
                        .bind_loc;

                    let err = Error::malformed("component provided phantom event binding to non-phantom event argument")
                    .add_note(self.diag.add_info("invoke provides phantom event", eb_info))
                    .add_note(self.diag.add_info("event is a phantom event", phantom_info))
                    .add_note(self.diag.add_info("instance's event is not phantom", inst_ev_info))
                    .add_note(self.diag.add_message("phantom ports are compiled away and cannot be used by subcomponents"));
                    self.diag.add_error(err);
                    self.diag_count += 1;
                    return Action::Stop;
                }
            }
        }
        Action::Continue
    }

    fn after_traversal(&mut self) -> Option<u32> {
        if self.diag_count > 0 {
            Some(self.diag_count)
        } else {
            None
        }
    }
}
