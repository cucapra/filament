use crate::{
    ast::param as ast,
    core::{Id, WithTime},
    errors::FilamentResult,
    visitor,
};
use itertools::Itertools;
use std::collections::HashMap;

type States = HashMap<Id, u64>;

#[derive(Default)]
pub struct MaxStates {
    /// Map for each event to the maximum number of states for each component
    pub max_states: HashMap<Id, States>,
    /// Current set of states we're working on
    cur_states: States,
}

impl MaxStates {
    fn max_state_from_ports(
        &mut self,
        resolved_outputs: impl Iterator<Item = ast::PortDef>,
    ) {
        let out_events = resolved_outputs.flat_map(|pd: ast::PortDef| {
            pd.liveness.events().into_iter().cloned().collect_vec()
        });

        // Use all ranges to compute max state
        out_events.for_each(|time| {
            let ev = &time.event;
            let v = self.cur_states.get_mut(ev).unwrap();
            let st = time.offset();
            if *v < st {
                *v = st;
            }
        });
    }
}

impl visitor::Transform for MaxStates {
    type Info = ();

    fn new(_: &ast::Namespace, _: &Self::Info) -> Self {
        Self::default()
    }
    fn clear_data(&mut self) {
        self.cur_states.clear();
    }
    fn component_filter(&self, _: &ast::Component) -> bool {
        true
    }
    fn enter_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        self.cur_states = comp
            .sig
            .events
            .iter()
            .map(|eb| (eb.event.clone(), 0))
            .collect();
        self.max_state_from_ports(comp.sig.inputs().cloned());
        Ok(comp)
    }
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        sig: &ast::ResolvedInstance,
    ) -> FilamentResult<Vec<ast::Command>> {
        let sig = sig.resolve()?;
        // Get the signature associated with this instance.
        let binding = sig.binding(&inv.abstract_vars);
        self.max_state_from_ports(sig.outputs().map(|pd| pd.resolve(&binding)));
        Ok(vec![ast::Command::Invoke(inv)])
    }
    fn exit_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        let events = std::mem::take(&mut self.cur_states);
        self.max_states.insert(comp.sig.name.clone(), events);
        Ok(comp)
    }
}
