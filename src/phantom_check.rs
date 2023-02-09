use crate::ast::param as ast;
use crate::core;
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor;
use std::collections::HashSet;

#[derive(Default)]
/// Checks if a user-level phantom events are valid.
/// Phantom events are valid iff:
/// 1. The component doesn't share any instances
/// 2. The component doesn't use an subcomponents that need to use the
///    corresponding event in their interface, i.e., the uses of the event are all phantom
pub struct PhantomCheck {
    // Set of instances that have already been used once
    instance_used: HashSet<ast::Id>,
    // Names of @phantom events in this component
    phantom_events: Vec<ast::Id>,
}

impl visitor::Transform<core::Time<u64>, core::PortParam> for PhantomCheck {
    type Info = ();
    fn new(_: &ast::Namespace, _: &Self::Info) -> Self {
        Self::default()
    }

    fn clear_data(&mut self) {
        self.instance_used.clear();
        self.phantom_events.clear();
    }

    // Only check component if at least one phantom event
    fn component_filter(&self, comp: &ast::Component) -> bool {
        comp.sig.phantom_events().next().is_some()
    }

    fn enter_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        self.phantom_events = comp.sig.phantom_events().collect();
        Ok(comp)
    }

    fn invoke(
        &mut self,
        inv: ast::Invoke,
        resolved: &ast::ResolvedInstance,
    ) -> FilamentResult<Vec<ast::Command>> {
        // Check if the instance has already been used
        if let Some(prev_use) = self.instance_used.get(&inv.instance) {
            for ev in inv.abstract_vars.iter().map(|ev| &ev.event) {
                if let Some(e) = self.phantom_events.iter().find(|e| *e == ev) {
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
        let instance_phantoms = resolved.phantom_events();
        for (eb, bind) in resolved
            .abstract_vars()
            .iter()
            .zip(inv.abstract_vars.iter())
        {
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !instance_phantoms.contains(&eb.event) {
                let ev = &bind.event;
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

        Ok(vec![inv.into()])
    }
}
