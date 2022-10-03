use std::collections::HashMap;

use crate::errors::{self, FilamentResult, WithPos};
use crate::event_checker::ast;
use crate::visitor;

#[derive(Default)]
pub struct DumpInterface {
    /// Map from FSM trigger to number of states
    fsm_states: HashMap<ast::Id, u64>,
}

impl visitor::Transform for DumpInterface {
    fn new(_: &ast::Namespace) -> Self {
        Self::default()
    }

    fn component_filter(&self, comp: &ast::Component) -> bool {
        comp.sig.name == "main"
    }

    fn fsm(&mut self, fsm: ast::Fsm) -> FilamentResult<Vec<ast::Command>> {
        self.fsm_states
            .insert(fsm.trigger.name().clone(), fsm.states);
        Ok(vec![fsm.into()])
    }

    fn exit_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        // For an interface port like this:
        //      @interface[G, G+5] go_G
        // Generate the JSON information:
        // {
        //   "name": "go_G",
        //   "event": "G",
        //   "delay": 2,
        //   "states": 5,
        //   "phantom": false
        // }
        let interfaces = comp
        .sig
        .interface_signals
        .iter()
        .map(|id| {
            let states = self.fsm_states[&id.name];
            id.delay()
                .concrete()
                .map(|delay|
                    format!(
                        "{{\"name\": \"{}\", \"event\": \"{}\", \"delay\": {delay}, \"states\": {states}, \"phantom\": {} }}",
                        id.name,
                        id.event,
                        id.phantom
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
        let pd_to_info = |pd: &ast::PortDef<u64>| {
            let w = pd.bitwidth;
            pd.liveness
            .within
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
            .inputs
            .iter()
            .map(pd_to_info)
            .collect::<FilamentResult<Vec<_>>>()?
            .join(",\n");
        let outputs = comp
            .sig
            .outputs
            .iter()
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
