use crate::core;
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor;
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
}

impl visitor::Checker for PhantomCheck {
    fn new(_: &core::Namespace) -> FilamentResult<Self> {
        Ok(Self {
            instance_used: HashSet::new(),
            phantom_events: Vec::new(),
        })
    }

    fn clear_data(&mut self) {
        self.instance_used.clear();
        self.phantom_events.clear();
    }

    // Only check component if at least one phantom event
    fn component_filter(&self, comp: &core::Component) -> bool {
        comp.sig.phantom_events().next().is_some()
    }

    fn enter_component(
        &mut self,
        comp: &core::Component,
        _: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        self.phantom_events = comp.sig.phantom_events().collect();
        Ok(())
    }

    fn invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        // Check if the instance has already been used
        if let Some(prev_use) = self.instance_used.get(&inv.instance) {
            for ev in inv.abstract_vars.iter().map(|ev| ev.event()) {
                if let Some(e) = self.phantom_events.iter().find(|e| **e == ev)
                {
                    return Err(Error::malformed(
                        "Reuses instance uses phantom event for scheduling"
                    ).add_note("Invocation uses phantom event", ev.copy_span())
                     .add_note("Event is a phantom event", e.copy_span())
                     .add_note("Previous use", prev_use.copy_span())
                     .add_note("Phantom ports are compiled away and cannot be used for resource sharing", GPosIdx::UNKNOWN));
                }
            }
        }
        self.instance_used.insert(inv.instance.clone());

        // For each binding provided to a non-phantom port, check that the
        // mentioned events are not non-phantom
        let inv_idx = ctx.get_invoke_idx(&inv.name).unwrap();
        let sig = inv_idx.unresolved_signature(ctx);
        let instance_phantoms = ctx.prog.phantom_events(sig);
        for (eb, bind) in ctx
            .prog
            .event_names(sig)
            .iter()
            .zip(inv.abstract_vars.iter())
        {
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !instance_phantoms.contains(eb) {
                let ev = &bind.event();
                if let Some(e) = self.phantom_events.iter().find(|e| *e == ev) {
                    return Err(Error::malformed(
                            "Component provided phantom event binding to non-phantom event argument",
                        ).add_note("Invoke provides phantom event", ev.copy_span())
                         .add_note("Event is a phantom event", e.copy_span())
                         .add_note("Instance's event is not phantom", eb.copy_span())
                         .add_note("Phantom ports are compiled away and cannot be used by subcomponents", GPosIdx::UNKNOWN));
                }
            }
        }

        Ok(())
    }
}
