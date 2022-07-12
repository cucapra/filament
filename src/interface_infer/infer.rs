//! Infer @interface signals for all events in Filament-level modules.
//! Once this pass is run, all Filament-level components are guaranteed to have @interface defined
//! for all the events they use.

use crate::errors::FilamentResult;
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
    fn new(_: &ast::Namespace) -> Self {
        Self::default()
    }

    // Visit this component if it doesn't have interface ports defined. Otherwise, assumes that
    // this is a low-level component.
    fn component_filter(&self, comp: &ast::Component) -> bool {
        comp.sig.interface_signals.is_empty()
    }

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
        // Add interface ports for all components
        let missing_interfaces = comp.sig.abstract_vars.iter().map(|ev| {
            ast::InterfaceDef::new(
                format!("go_{}", ev).into(),
                ev.clone(),
                self.max_states[ev],
            )
        });
        comp.sig.interface_signals.extend(missing_interfaces);

        Ok(comp)
    }
}
