//! Infer @interface signals for all events in Filament-level modules

use crate::errors::{Error, FilamentResult};
use crate::event_checker::ast;
use crate::visitor;
use itertools::Itertools;
use std::collections::HashMap;

/// Infer @interface ports for events defined in a component.
/// Walks over each component and computes the maximum observed event for each event.
/// Events usages are only considered in other invokes.
#[derive(Default)]
struct InterfaceInfer {
    /// Maximum state used by an event variable.
    pub max_states: HashMap<ast::Id, u64>,
}

impl InterfaceInfer {
    /// Compute max states from:
    /// 1. Inputs and output ports
    /// 2. The events used to trigger the invocation
    fn max_state_from_sig(
        &mut self,
        sig: &ast::Signature,
        abstract_vars: &[ast::TimeRep],
        binding: &HashMap<ast::Id, &ast::TimeRep>,
    ) {
        let out_events =
            sig.outputs.iter().chain(sig.inputs.iter()).flat_map(|pd| {
                pd.liveness
                    .resolve(binding)
                    .events()
                    .into_iter()
                    .cloned()
                    .collect_vec()
            });

        // Abstract variables can affect the max state calculation
        let abs_events = abstract_vars.iter().cloned();

        // Use all ranges to compute max state
        out_events.chain(abs_events).for_each(|fsm| {
            fsm.events().for_each(|(ev, &st)| {
                if self.max_states[ev] < st {
                    *self.max_states.get_mut(ev).unwrap() = st;
                }
            })
        });
    }
}

impl visitor::Transform for InterfaceInfer {
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        sig: &ast::Signature,
    ) -> FilamentResult<Vec<ast::Command>> {
        // Compute maximum events observed in this
        let bindings = inv.bindings(sig);
        self.max_state_from_sig(sig, &inv.abstract_vars, &bindings);
        Ok(vec![inv.into()])
    }

    fn exit_component(
        &mut self,
        mut comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        // Add missing interface ports
        let missing_events = comp.sig.missing_interface_ports();
        let missing_interfaces = missing_events.iter().map(|ev| {
            ast::InterfaceDef::new(
                format!("go_{}", ev).into(),
                ev.clone(),
                self.max_states[ev],
            )
        });
        comp.sig.interface_signals.extend(missing_interfaces);

        // For all the defined interfaces make sure that they match what we inferred.
        comp.sig.interface_signals.iter().try_for_each(|id| {
            if id.delay() != self.max_states[&id.event] {
                Err(Error::malformed(format!(
                    "Interface signal has delay {} but compiler infers {}",
                    id.delay(),
                    self.max_states[&id.event]
                )))
            } else {
                Ok(())
            }
        })?;

        Ok(comp)
    }
}
