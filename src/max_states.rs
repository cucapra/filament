use crate::{
    core::{self, Id},
    errors::FilamentResult,
    visitor,
};
use std::collections::HashMap;

type States = HashMap<Id, u64>;

/// Compute the maximum number of states for each event in a component
#[derive(Default)]
pub struct MaxStates {
    /// Map for each event to the maximum number of states for each component
    pub max_states: HashMap<Id, States>,
    /// Current set of states we're working on
    cur_states: States,
}

impl MaxStates {
    fn max_state_from_ports<'a>(
        &mut self,
        ports: impl IntoIterator<Item = &'a core::PortDef>,
    ) {
        for pd in ports {
            for time in pd.liveness.time_exprs() {
                let ev = &time.event;
                let v = self.cur_states.get_mut(ev).unwrap();
                let st = time.offset().concrete();
                if *v < st {
                    *v = st;
                }
            }
        }
    }
}

impl visitor::Checker for MaxStates {
    fn new(_: &core::Namespace) -> FilamentResult<Self> {
        Ok(Self {
            max_states: HashMap::new(),
            cur_states: HashMap::new(),
        })
    }
    fn clear_data(&mut self) {
        self.cur_states.clear();
    }
    fn enter_component(
        &mut self,
        comp: &core::Component,
        _: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        self.cur_states = comp
            .sig
            .events
            .iter()
            .map(|eb| (eb.event.clone(), 0))
            .collect();
        self.max_state_from_ports(comp.sig.inputs());
        Ok(())
    }
    fn invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        // Get the fully resolved signature
        let inv_idx = ctx.get_invoke_idx(&inv.name).unwrap();
        let sig = inv_idx.resolved_signature(ctx);
        self.max_state_from_ports(sig.outputs());
        Ok(())
    }
    fn exit_component(
        &mut self,
        comp: &core::Component,
        _: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        let events = std::mem::take(&mut self.cur_states);
        log::info!("Max states for {}: {:?}", comp.sig.name, events);
        self.max_states.insert(comp.sig.name.clone(), events);
        Ok(())
    }
}
