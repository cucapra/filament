use crate::errors::{Error, WithPos};
use crate::utils::GPosIdx;
use crate::visitor::{self, Traverse};
use crate::{binding, core, diagnostics};
use itertools::Itertools;
use std::collections::HashSet;

/// Checks if a user-level phantom events are valid.
/// Phantom events are valid iff:
/// 1. The component doesn't share any instances
/// 2. The component doesn't use an subcomponents that need to use the
///    corresponding event in their interface, i.e., the uses of the event are all phantom
pub struct PhantomCheck {
    // Set of instances that have already been used once
    instance_used: HashSet<core::Id>,
    // Names of @phantom events in this component
    phantom_events: Vec<core::Id>,
    // Diagnostics information
    diag: diagnostics::Diagnostics,
}

impl visitor::Checker for PhantomCheck {
    fn new(_: &core::Namespace) -> Self {
        Self {
            instance_used: HashSet::new(),
            phantom_events: Vec::new(),
            diag: diagnostics::Diagnostics::default(),
        }
    }

    fn clear_data(&mut self) {
        self.instance_used.clear();
        self.phantom_events.clear();
    }

    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    fn enter_component(
        &mut self,
        comp: &core::Component,
        _: &binding::CompBinding,
    ) -> Traverse {
        self.phantom_events = comp.sig.phantom_events().collect();
        if self.phantom_events.is_empty() {
            Traverse::Break(())
        } else {
            Traverse::Continue(())
        }
    }

    fn invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        // Check if the instance has already been used
        if let Some(prev_use) = self.instance_used.get(&inv.instance) {
            for ev in inv.abstract_vars.iter().map(|ev| ev.event()) {
                if let Some(e) = self.phantom_events.iter().find(|e| **e == ev)
                {
                    let err =  Error::malformed(
                        "reuses instance uses phantom event for scheduling"
                    ).add_note(self.diag.add_info("invocation uses phantom event", ev.copy_span()))
                     .add_note(self.diag.add_info("event is a phantom event", e.copy_span()))
                     .add_note(self.diag.add_info("previous use", prev_use.copy_span()))
                     .add_note(self.diag.add_info("phantom ports are compiled away and cannot be used for resource sharing", GPosIdx::UNKNOWN));
                    self.diag.add_error(err);
                }
            }
        }
        self.instance_used.insert(inv.instance.clone());

        // For each binding provided to a non-phantom port, check that the
        // mentioned events are not non-phantom
        let inv_idx = ctx.get_invoke_idx(&inv.name);
        let sig = inv_idx.unresolved_signature(ctx);
        let instance_phantoms = ctx.prog[sig].phantom_events().collect_vec();
        for (event, bind) in
            ctx.prog[sig].events().zip(inv.abstract_vars.iter())
        {
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !instance_phantoms.contains(&event) {
                let ev = &bind.event();
                if let Some(e) = self.phantom_events.iter().find(|e| *e == ev) {
                    let err = Error::malformed(
                        "component provided phantom event binding to non-phantom event argument",
                    ).add_note(self.diag.add_info("invoke provides phantom event", ev.copy_span()))
                    .add_note(self.diag.add_info("event is a phantom event", e.copy_span()))
                    .add_note(self.diag.add_info("instance's event is not phantom", event.copy_span()))
                    .add_note(self.diag.add_message("phantom ports are compiled away and cannot be used by subcomponents"));
                    self.diag.add_error(err);
                }
            }
        }

        Traverse::Continue(())
    }
}
