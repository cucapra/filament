use crate::{
    core::{self, Id, Time, WidthRep, WithTime},
    errors::FilamentResult,
    visitor,
};
use itertools::Itertools;
use std::collections::HashMap;

type States = HashMap<Id, u64>;

/// Compute the maximum number of states for each event in a component
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
        resolved_outputs: impl IntoIterator<Item = core::PortDef<Time<u64>, W>>,
    ) {
        let out_events = resolved_outputs.into_iter().flat_map(
            |pd: core::PortDef<Time<u64>, W>| {
                pd.liveness.events().into_iter().cloned().collect_vec()
            },
        );

        // Use all ranges to compute max state
        out_events.for_each(|time| {
            let ev = &time.event;
            let v = self.cur_states.get_mut(ev).unwrap();
            let st = time.offset().concrete();
            if *v < st {
                *v = st;
            }
        });
    }
}

impl<W: WidthRep> visitor::Checker<core::Time<u64>, W> for MaxStates<W> {
    fn new(_: &core::Namespace<Time<u64>, W>) -> FilamentResult<Self> {
        Ok(Self {
            max_states: HashMap::new(),
            cur_states: HashMap::new(),
            w: std::marker::PhantomData,
        })
    }
    fn clear_data(&mut self) {
        self.cur_states.clear();
    }
    fn enter_component(
        &mut self,
        comp: &core::Component<Time<u64>, W>,
        _: &visitor::CompBinding<Time<u64>, W>,
    ) -> FilamentResult<()> {
        self.cur_states = comp
            .sig
            .events
            .iter()
            .map(|eb| (eb.event.clone(), 0))
            .collect();
        self.max_state_from_ports(comp.sig.inputs().cloned());
        Ok(())
    }
    fn invoke(
        &mut self,
        inv: &core::Invoke<Time<u64>>,
        ctx: &visitor::CompBinding<Time<u64>, W>,
    ) -> FilamentResult<()> {
        let inst = ctx.get_instance(&inv.instance);
        let outputs = ctx.prog.output_names(inst.sig);
        let ports = outputs.into_iter().map(|port| {
            ctx.get_invoke_port(&inv.name, port, |range, event_b, _| {
                range.resolve_event(event_b)
            })
            .unwrap()
        });

        // Get the signature associated with this instance.
        self.max_state_from_ports(ports);
        Ok(())
    }
    fn exit_component(
        &mut self,
        comp: &core::Component<Time<u64>, W>,
        _: &visitor::CompBinding<Time<u64>, W>,
    ) -> FilamentResult<()> {
        let events = std::mem::take(&mut self.cur_states);
        self.max_states.insert(comp.sig.name.clone(), events);
        Ok(())
    }
}
