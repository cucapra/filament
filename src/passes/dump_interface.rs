use crate::core;
use crate::errors::{self, FilamentResult};
use crate::visitor::{self, CompBinding};
use itertools::Itertools;
use std::collections::HashMap;

pub struct DumpInterface {
    /// Map from component to interface information
    max_states: HashMap<core::Id, HashMap<core::Id, u64>>,
}

impl visitor::Transform for DumpInterface {
    // Mapping from component -> event -> max state
    type Info = HashMap<core::Id, HashMap<core::Id, u64>>;

    fn new(_: &core::Namespace, max_states: &Self::Info) -> Self {
        Self {
            max_states: max_states.clone(),
        }
    }

    fn clear_data(&mut self) {}

    fn component_filter(&self, comp: &visitor::CompBinding) -> bool {
        let sig = comp.this();
        sig.name == "main"
    }

    fn exit_component(
        &mut self,
        comp: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        let sig = comp.this();

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
        let events = &self.max_states[&sig.name];
        let interfaces = sig
            .events
            .iter()
            .map(|eb| {
                let id = sig.get_interface(&eb.event);
                let phantom = id.is_none();
                eb.delay
                    .concrete()
                    .map(|delay|
                        format!(
                            "{{\"name\": {}, \"event\": \"{}\", \"delay\": {}, \"states\": {}, \"phantom\": {} }}",
                            id.map(|i| format!("\"{}\"", i.name.as_ref())).unwrap_or_else(|| "null".to_string()),
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
        let pd_to_info = |pd: &core::PortDef| {
            let w = &pd.bitwidth;
            let (event, st, end) = pd.liveness
            .as_offset()
            .unwrap_or_else(|| unreachable!("Cannot covert delay into concrete start and end. Resolved delays should use the same event."));
            format!(
                "{{ \"event\": \"{event}\", \"name\": \"{name}\", \"width\": {w} , \"start\": {st}, \"end\": {end} }}",
                name = pd.name,
                st = st.concrete().unwrap(),
                end = end.concrete().unwrap(),
            )
        };

        let inputs = sig.inputs().map(pd_to_info).collect_vec().join(",\n");
        let outputs = sig.outputs().map(pd_to_info).collect_vec().join(",\n");

        // Look ma, a JSON serializer!
        println!(
            "{{\n\"interfaces\": [\n{interfaces}\n],\n\"inputs\": [\n{inputs}\n],\n\"outputs\": [\n{outputs}\n]\n}}",
        );

        Ok(vec![])
    }
}
