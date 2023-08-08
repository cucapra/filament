use crate::{
    diagnostics,
    errors::Error,
    ir::{self, Ctx},
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
        let inst = data.comp.get(inv).inst;
        if let Some(prev_use) = self.instance_used.get(&inst) {
            for time in inv.times(&data.comp) {
                if let Some(e) = self
                    .phantom_events
                    .iter()
                    .find(|e| time.event(&data.comp) == **e)
                {
                    let err =
                        Error::malformed("reuses phantom event for scheduling");
                    self.diag.add_error(err);
                }
            }
        }
        self.instance_used.insert(inst);

        Action::Continue
    }
}
