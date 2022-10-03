use crate::errors::{Error, FilamentResult, WithPos};
use crate::event_checker::ast;
use crate::visitor;
use std::collections::HashSet;

#[derive(Default)]
/// Checks if a user-level @inteface port marked as @phantom is valid.
/// Such ports are valid iff:
/// 1. The component doesn't share any instances
/// 2. The component doesn't use an subcomponents that need to use the
///    corresponding event in their interface, i.e., the uses of the event are all
///    to other phantom<'a> {
pub struct PhantomCheck {
    // Set of instances that have already been used once
    instance_used: HashSet<ast::Id>,
    // Names of @phantom events in this component
    phantom_events: HashSet<ast::Id>,
}

impl visitor::Transform for PhantomCheck {
    // Only check component if at least one @interface port is @phantom
    fn component_filter(&self, comp: &ast::Component) -> bool {
        comp.sig
            .interface_signals
            .iter()
            .any(|interface| interface.phantom)
    }

    fn enter_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        self.phantom_events = comp
            .sig
            .interface_signals
            .iter()
            .filter_map(|interface| {
                if interface.phantom {
                    Some(interface.event.clone())
                } else {
                    None
                }
            })
            .collect();
        Ok(comp)
    }

    fn invoke(
        &mut self,
        inv: ast::Invoke,
        resolved: &visitor::ResolvedInstance,
    ) -> FilamentResult<Vec<ast::Command>> {
        // Check if the instance has already been used
        if let Some(prev_use) = self.instance_used.get(&inv.instance) {
            for (ev, _) in inv.abstract_vars.iter().flat_map(|ev| ev.events()) {
                if let Some(interface_ev) = self.phantom_events.get(&ev) {
                    return Err(Error::malformed(
                        "Reuses instance uses phantom event for scheduling"
                    ).add_note("Invocation uses phantom port", ev.copy_span())
                     .add_note("Event defined as phantom", interface_ev.copy_span())
                     .add_note("Previous use", prev_use.copy_span())
                     .add_note("Phantom ports are compiled away and cannot be used for resource sharing", None));
                }
            }
        }
        self.instance_used.insert(inv.instance.clone());

        // For each binding provided to a non-phantom port, check that the
        // mentioned events are not non-phantom
        let instance_interfaces = resolved.interface_signals();
        for (eb, bind) in resolved
            .abstract_vars()
            .iter()
            .zip(inv.abstract_vars.iter())
        {
            let id = instance_interfaces
                .iter()
                .find(|id| id.event == eb.event)
                .unwrap();
            // If this event is non-phantom, ensure all provided events are non-phantom as well.
            if !id.phantom {
                for (ev, _) in bind.events() {
                    if let Some(interface_ev) = self.phantom_events.get(ev) {
                        return Err(Error::malformed(
                            "Component provided phantom event to non-phantom port",
                        ).add_note("Invoke provides phantom event to non-phantom port", ev.copy_span())
                         .add_note("Instance's signature defines this port to be concrete", id.copy_span())
                         .add_note("This event is defined to be @phantom", interface_ev.copy_span())
                         .add_note("Phantom ports are compiled away and cannot be used by subcomponents", None));
                    }
                }
            }
        }

        Ok(vec![inv.into()])
    }

    fn new(_: &ast::Namespace) -> Self {
        Self::default()
    }
}
