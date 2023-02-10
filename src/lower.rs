use crate::core::{self, Id, Time, TimeRep, WidthRep, WithTime};
use crate::errors::{FilamentResult, WithPos};
use crate::visitor;
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileInvokes<W: WidthRep> {
    /// Mapping from events to FSMs
    fsms: HashMap<core::Id, core::Fsm>,
    /// Max state map
    max_states: HashMap<core::Id, HashMap<core::Id, u64>>,
    /// Phantom data for time and width
    _tw: std::marker::PhantomData<W>,
}

impl<W: WidthRep> CompileInvokes<W> {
    fn find_fsm(&self, event: &core::Id) -> Option<&core::Fsm> {
        self.fsms.get(event)
    }

    fn get_fsm(&self, event: &core::Id) -> &core::Fsm {
        self.find_fsm(event)
            .unwrap_or_else(|| panic!("No FSM for event `{event}`."))
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    fn range_to_guard(
        &self,
        range: core::Range<Time<u64>>,
    ) -> Option<core::Guard> {
        let Some((ev, st, end)) = range.as_offset() else {
            unreachable!(
                "Range `{range}` cannot be represented as a simple offset"
            )
        };

        let fsm = self.find_fsm(&ev)?;
        let guard = (st.concrete()..end.concrete())
            .into_iter()
            .map(|st| fsm.port(st).into())
            .reduce(core::Guard::or)
            .unwrap();
        Some(guard)
    }
}

impl<W: WidthRep> visitor::Transform<Time<u64>, W> for CompileInvokes<W> {
    /// Mapping from component -> event -> max state
    type Info = HashMap<Id, HashMap<Id, u64>>;

    fn new(_: &core::Namespace<Time<u64>, W>, max_states: &Self::Info) -> Self {
        Self {
            fsms: HashMap::new(),
            max_states: max_states.clone(),
            _tw: std::marker::PhantomData,
        }
    }

    fn clear_data(&mut self) {
        self.fsms.clear();
    }

    /// Visit components with high-level invokes
    fn component_filter(&self, comp: &core::Component<Time<u64>, W>) -> bool {
        comp.body.iter().any(|con| {
            if let core::Command::Invoke(core::Invoke { ports, .. }) = con {
                ports.is_some()
            } else {
                false
            }
        })
    }

    // TODO(rachit): Document how the compilation works
    fn invoke(
        &mut self,
        inv: core::Invoke<Time<u64>>,
        sig: &visitor::ResolvedInstance<Time<u64>, W>,
    ) -> FilamentResult<Vec<core::Command<Time<u64>, W>>> {
        let pos = inv.copy_span();
        // Compile only if this is a high-level invoke
        if let core::Invoke {
            name: bind,
            instance,
            abstract_vars,
            ports: Some(ports),
            ..
        } = inv
        {
            let sig = sig.resolve()?;
            // Get the signature associated with this instance.
            let binding = sig.event_binding(&abstract_vars);

            let mut connects = Vec::with_capacity(
                1 + ports.len() + sig.interface_signals.len(),
            );

            // Define the low-level invoke
            let low_inv =
                core::Invoke::new(bind.clone(), instance, abstract_vars, None)
                    .set_span(pos)
                    .into();
            connects.push(low_inv);

            // Generate the assignment for each interface port
            for interface in &sig.interface_signals {
                let ev = &interface.event;
                // Get binding for this event in the invoke
                let t = binding.get(ev);
                let start_time = t.offset().concrete();
                let port = self.get_fsm(&t.event()).port(start_time);
                let con = core::Connect::new(
                    core::Port::comp(bind.clone(), interface.name.clone()),
                    port,
                    None,
                )
                .set_span(pos);
                connects.push(con.into())
            }

            // Generate assignment for each port
            for (port, formal) in ports.into_iter().zip(sig.inputs()) {
                let req = formal.liveness.resolve_event(&binding);
                let guard = self.range_to_guard(req);
                let sp = port.copy_span();
                let con = core::Connect::new(
                    core::Port::comp(bind.clone(), formal.name.clone()),
                    port,
                    guard,
                )
                .set_span(sp);
                connects.push(con.into());
            }
            Ok(connects)
        } else {
            Ok(vec![inv.into()])
        }
    }

    /// Computes the max state traversed by each event variable
    fn enter_component(
        &mut self,
        comp: core::Component<Time<u64>, W>,
    ) -> FilamentResult<core::Component<Time<u64>, W>> {
        // Define FSMs for each interface signal
        let events = &self.max_states[&comp.sig.name];
        self.fsms = comp
            .sig
            .interface_signals
            .iter()
            .map(|interface| {
                let ev = &interface.event;
                Ok((
                    ev.clone(),
                    core::Fsm::new(
                        format!("{}_fsm", ev).into(),
                        events[ev],
                        core::Port::this(interface.name.clone()),
                    ),
                ))
            })
            .collect::<FilamentResult<HashMap<_, _>>>()?;

        Ok(comp)
    }

    fn exit_component(
        &mut self,
        comp: core::Component<Time<u64>, W>,
    ) -> FilamentResult<core::Component<Time<u64>, W>> {
        // Add the FSMs to the component
        let mut comp = comp;
        let fsms = std::mem::take(&mut self.fsms)
            .into_values()
            .map(|f| f.into());
        comp.body = fsms.chain(comp.body.into_iter()).collect();
        Ok(comp)
    }
}
