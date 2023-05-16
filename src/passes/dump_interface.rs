use crate::ast;
use itertools::Itertools;
use std::collections::HashMap;

pub struct DumpInterface;

// Mapping from component -> event -> max state
type States = HashMap<ast::Id, HashMap<ast::Id, u64>>;

impl DumpInterface {
    /// Print out the interface of the main component in JSON format
    pub fn print(ns: &ast::Namespace, max_states: &States) {
        // Search for the main component
        let main = ns
            .components
            .iter()
            .find(|c| c.sig.name.inner() == "main")
            .unwrap_or_else(|| panic!("Component named `main' not found"));

        let sig = &main.sig;

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
        let events = &max_states[&sig.name];
        let interfaces = sig
            .events
            .iter()
            .map(|eb| {
                let id = sig.get_interface(&eb.event);
                let phantom = id.is_none();
                let delay = eb.delay .concrete() .unwrap();
                format!(
                    "{{\"name\": {}, \"event\": \"{}\", \"delay\": {}, \"states\": {}, \"phantom\": {} }}",
                    id.map(|i| format!("\"{}\"", i.name.as_ref())).unwrap_or_else(|| "null".to_string()),
                    eb.event,
                    delay,
                    events[&eb.event],
                    phantom
                )
            })
            .collect_vec().join(",\n");

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
        let pd_to_info = |pd: &ast::Loc<ast::PortDef>| {
            let w = &pd.bitwidth();
            let (event, st, end) = pd.liveness()
            .as_offset()
            .unwrap_or_else(|| unreachable!("Cannot covert delay into concrete start and end. Resolved delays should use the same event."));
            format!(
                "{{ \"event\": \"{event}\", \"name\": \"{name}\", \"width\": {w} , \"start\": {st}, \"end\": {end} }}",
                name = pd.name(),
                st = u64::try_from(&st).unwrap(),
                end = u64::try_from(&end).unwrap(),
            )
        };

        let inputs = sig.inputs().map(pd_to_info).collect_vec().join(",\n");
        let outputs = sig.outputs().map(pd_to_info).collect_vec().join(",\n");

        // Look ma, a JSON serializer!
        println!(
            "{{\n\"interfaces\": [\n{interfaces}\n],\n\"inputs\": [\n{inputs}\n],\n\"outputs\": [\n{outputs}\n]\n}}",
        );
    }
}
