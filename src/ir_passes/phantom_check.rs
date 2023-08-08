use itertools::Itertools;

use crate::{
    diagnostics,
    errors::Error,
    ir::{self, Context, Ctx},
    ir_visitor::{Action, Construct, Visitor, VisitorData},
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
    phantom_events: Vec<ir::EventIdx>,
    diag: diagnostics::Diagnostics,
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
        if let Some(prev_use) = self.instance_used.get(&inst) {
            for (time, info) in inv.times(comp) {
                if let Some(e) =
                    self.phantom_events.iter().find(|e| time.event(comp) == **e)
                {
                    let err =
                        Error::malformed("reuses phantom event for scheduling");
                    self.diag.add_error(err);
                }
            }
        }
        self.instance_used.insert(inst);

        // For each binding provided to a non-phantom port, check that the
        // mentioned events are not non-phantom
        let inst_comp = ctx.get(comp.get(inst).comp);
        let inst_phantoms = inst_comp.phantom_events().collect_vec();
        for (event, (bind, _)) in
            inst_comp.events().idx_iter().zip(inv.times(comp).iter())
        {
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !inst_phantoms.contains(&event) {
                let ev = &bind.event(comp);
                if let Some(_) = self.phantom_events.iter().find(|e| **e == *ev)
                {
                    let err = Error::malformed("component provided phantom event binding to non-phantom event argument");
                    self.diag.add_error(err);
                }
            }
        }
        Action::Continue
    }
}
