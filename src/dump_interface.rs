use crate::core::{self, Time, WidthRep};
use crate::errors::{self, FilamentResult, WithPos};
use crate::visitor;
use std::collections::HashMap;

pub struct DumpInterface<W: WidthRep> {
    /// Map from FSM trigger to number of states
    fsm_states: HashMap<core::Id, u64>,
    /// Map from component to interface information
    max_states: HashMap<core::Id, HashMap<core::Id, u64>>,
    /// Phantom data for width
    _w: std::marker::PhantomData<W>,
}

impl<W: WidthRep> visitor::Transform<Time<u64>, W> for DumpInterface<W> {
    // Mapping from component -> event -> max state
    type Info = HashMap<core::Id, HashMap<core::Id, u64>>;

    fn new(_: &core::Namespace<Time<u64>, W>, max_states: &Self::Info) -> Self {
        Self {
            fsm_states: HashMap::new(),
            max_states: max_states.clone(),
            _w: std::marker::PhantomData,
        }
    }

    fn clear_data(&mut self) {
        self.fsm_states.clear();
    }

    fn component_filter(&self, comp: &core::Component<Time<u64>, W>) -> bool {
        comp.sig.name == "main"
    }

    fn fsm(
        &mut self,
        fsm: core::Fsm,
    ) -> FilamentResult<Vec<core::Command<Time<u64>, W>>> {
        self.fsm_states
            .insert(fsm.trigger.name().clone(), fsm.states);
        Ok(vec![fsm.into()])
    }

    fn exit_component(
        &mut self,
        comp: core::Component<Time<u64>, W>,
    ) -> FilamentResult<core::Component<Time<u64>, W>> {
        // For an interface port like this:
        //      @interface[G, G+5] go_G
        // Generate the JSON information:
        // {
        //   "name": "go_G",
        //   "event": "G",
        //   "delay": 5,
        //   "states": 2,
        //   "phantom": false
        // }
        let events = &self.max_states[&comp.sig.name];
        let interfaces = comp
            .sig
            .events
            .iter()
            .map(|eb| {
                let id = comp.sig.get_interface(&eb.event);
                let phantom = id.is_none();
                eb.delay
                    .concrete()
                    .map(|delay|
                        format!(
                            "{{\"name\": {}, \"event\": \"{}\", \"delay\": {}, \"states\": {}, \"phantom\": {} }}",
                            id.map(|i| format!("\"{}\"", i.name.id())).unwrap_or_else(|| "null".to_string()),
                            eb.event,
                            delay,
                            events[&eb.event],
                            phantom
                        ))
                    .ok_or_else(|| {
                        errors::Error::malformed(
                            "Interface signal has no concrete delay",
                        )
                    })
            })
            .collect::<FilamentResult<Vec<_>>>()?.join(",\n");

        // For each input and output that looks like:
        // @[G+n, G+m] left: 32
        // Generate the JSON information:
        // {
        //   "event": "G",
        //   "name": "left",
        //   "width": 32,
        //   "start": n,
        //   "end": m
        // },
        let pd_to_info = |pd: &core::PortDef<Time<u64>, W>| {
            let w = &pd.bitwidth;
            pd.liveness
            .as_offset()
            .map(|(event, st, end)| {
                format!(
                    "{{ \"event\": \"{event}\", \"name\": \"{name}\", \"width\": {w} , \"start\": {st}, \"end\": {end} }}",
                    name = pd.name,
                )
            })
            .ok_or_else(|| {
                errors::Error::malformed(
                    "Delay cannot be converted into concrete start and end",
                )
                .add_note(
                    format!("Delay {} is dynamic", pd.liveness),
                    pd.liveness.copy_span(),
                )
            })
        };
        let inputs = comp
            .sig
            .inputs()
            .map(pd_to_info)
            .collect::<FilamentResult<Vec<_>>>()?
            .join(",\n");
        let outputs = comp
            .sig
            .outputs()
            .map(pd_to_info)
            .collect::<FilamentResult<Vec<_>>>()?
            .join(",\n");

        // Look ma, a JSON serializer!
        println!(
            "{{\n\"interfaces\": [\n{interfaces}\n],\n\"inputs\": [\n{inputs}\n],\n\"outputs\": [\n{outputs}\n]\n}}",
        );

        Ok(comp)
    }
}
