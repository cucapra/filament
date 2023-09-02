use crate::{
    ir::{self, Ctx, DisplayCtx},
    ir_passes::lower::max_states,
};
use itertools::Itertools;

pub struct DumpInterface;

impl DumpInterface {
    /// Print out the interface of the main component in JSON format
    pub fn print(ctx: &ir::Context) {
        let entrypoint = ctx
            .entrypoint
            .unwrap_or_else(|| panic!("No entrypoint found."));
        let main = ctx.get(entrypoint);
        let src_info = main
            .src_info
            .as_ref()
            .unwrap_or_else(|| panic!("No source info found for main."));

        // For an interface port like this:
        //      @interface['G, 'G+5] go_G
        // Generate the JSON information:
        // {
        //   "name": "go_G",
        //   "event": "G",
        //   "delay": 5,
        //   "states": 2,
        //   "phantom": false
        // }
        let states = max_states(main);
        let interfaces = main
            .events()
            .iter()
            .map(|(idx, ev)| {
                let id = src_info.interface_ports.get(&idx).map_or("null", |v| v.as_ref());
                let phantom = !ev.has_interface;
                let ir::TimeSub::Unit(delay) = ev.delay else {
                    panic!("Event `{}` has a non-simple delay.", main.display(idx));
                };
                let delay = delay.as_concrete(main).unwrap();

                format!(
                    "{{\"name\": \"{}\", \"event\": \"{}\", \"delay\": {}, \"states\": {}, \"phantom\": {} }}",
                    id,
                    src_info.events.get(&idx).unwrap(),
                    delay,
                    states[&idx],
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
        let pd_to_info = |(idx, p): (ir::PortIdx, &ir::Port)| {
            let w = p.width.as_concrete(main).unwrap();
            assert!(
                idx.is_not_bundle(main),
                "Main component cannot have bundle ports in signature."
            );
            let ir::Liveness { range, .. } = &p.live;
            let start = main.get(range.start);
            let end = main.get(range.end);

            assert!(
                start.event == end.event,
                "Range `{}` cannot be represented as a simple offset",
                main.display_range(range)
            );

            format!(
                "{{ \"event\": \"{event}\", \"name\": \"{name}\", \"width\": {w} , \"start\": {st}, \"end\": {end} }}",
                event = src_info.events.get(&start.event).unwrap(),
                name = src_info.ports.get(&idx).unwrap(),
                st = start.offset.as_concrete(main).unwrap(),
                end = end.offset.as_concrete(main).unwrap(),
            )
        };

        let inputs = main.inputs().map(pd_to_info).collect_vec().join(",\n");
        let outputs = main.outputs().map(pd_to_info).collect_vec().join(",\n");

        // Look ma, a JSON serializer!
        println!(
            "{{\n\"interfaces\": [\n{interfaces}\n],\n\"inputs\": [\n{inputs}\n],\n\"outputs\": [\n{outputs}\n]\n}}",
        );
    }
}
