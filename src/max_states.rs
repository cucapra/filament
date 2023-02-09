use crate::{
    core::{self, Id, Time, WidthRep, WithTime},
    errors::FilamentResult,
    visitor,
};
use itertools::Itertools;
use std::collections::HashMap;

type States = HashMap<Id, u64>;

#[derive(Default)]
pub struct MaxStates<W: WidthRep> {
    /// Map for each event to the maximum number of states for each component
    pub max_states: HashMap<Id, States>,
    /// Current set of states we're working on
    cur_states: States,
    w: std::marker::PhantomData<W>,
}

impl<W: WidthRep> MaxStates<W> {
    fn max_state_from_ports(
        &mut self,
        resolved_outputs: impl Iterator<Item = core::PortDef<Time<u64>, W>>,
    ) {
        let out_events =
            resolved_outputs.flat_map(|pd: core::PortDef<Time<u64>, W>| {
                pd.liveness.events().into_iter().cloned().collect_vec()
            });

        // Use all ranges to compute max state
        out_events.for_each(|time| {
            let ev = &time.event;
            let v = self.cur_states.get_mut(ev).unwrap();
            let st = time.offset();
            if *v < *st {
                *v = *st;
            }
        });
    }
}

impl<W: WidthRep> visitor::Transform<core::Time<u64>, W> for MaxStates<W> {
    type Info = ();

    fn new(_: &core::Namespace<Time<u64>, W>, _: &Self::Info) -> Self {
        Self {
            max_states: HashMap::new(),
            cur_states: HashMap::new(),
            w: std::marker::PhantomData,
        }
    }
    fn clear_data(&mut self) {
        self.cur_states.clear();
    }
    fn component_filter(&self, _: &core::Component<Time<u64>, W>) -> bool {
        true
    }
    fn enter_component(
        &mut self,
        comp: core::Component<Time<u64>, W>,
    ) -> FilamentResult<core::Component<Time<u64>, W>> {
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
        inv: core::Invoke<Time<u64>>,
        sig: &visitor::ResolvedInstance<Time<u64>, W>,
    ) -> FilamentResult<Vec<core::Command<Time<u64>, W>>> {
        let sig = sig.resolve()?;
        // Get the signature associated with this instance.
        let binding = sig.binding(&inv.abstract_vars);
        self.max_state_from_ports(sig.outputs().map(|pd| pd.resolve(&binding)));
        Ok(vec![core::Command::Invoke(inv)])
    }
    fn exit_component(
        &mut self,
        comp: core::Component<Time<u64>, W>,
    ) -> FilamentResult<core::Component<Time<u64>, W>> {
        let events = std::mem::take(&mut self.cur_states);
        self.max_states.insert(comp.sig.name.clone(), events);
        Ok(comp)
    }
}
