use crate::binding::CompBinding;
use crate::core::{self, Id, Loc};
use crate::errors::FilamentResult;
use crate::visitor;
use itertools::Itertools;
use std::collections::HashMap;

/// Compiles high-level invokes into low-level invokes by instantiating FSMs and generating guards.
/// Additionally removes all bundles and inlines their reads.
#[derive(Default)]
pub struct Lower {
    /// Mapping from events to FSMs
    fsms: HashMap<core::Id, core::Fsm>,
    /// Max state map
    max_states: HashMap<core::Id, HashMap<core::Id, u64>>,
    /// Track writes to bundles
    bundle_writes: HashMap<core::Id, Vec<Option<core::Port>>>,
}

impl Lower {
    fn find_fsm(&self, event: &core::Id) -> Option<&core::Fsm> {
        self.fsms.get(event)
    }

    fn get_fsm(&self, event: &core::Id) -> &core::Fsm {
        self.find_fsm(event)
            .unwrap_or_else(|| panic!("No FSM for event `{event}`."))
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    fn range_to_guard(&self, range: &core::Range) -> Option<core::Guard> {
        let Some((ev, st, end)) = range.as_offset() else {
            unreachable!(
                "Range `{range}` cannot be represented as a simple offset"
            )
        };

        let fsm = self.find_fsm(&ev)?;
        let guard = (st.try_into().unwrap()..end.try_into().unwrap())
            .map(|st| fsm.port(st).into())
            .reduce(core::Guard::or)
            .unwrap();
        Some(guard)
    }

    fn port(&self, port: &core::Port) -> core::Port {
        match &port {
            core::Port::Bundle { name, idx } => {
                let idx = u64::try_from(idx.inner()).unwrap() as usize;
                let writes = self.bundle_writes.get(name).unwrap();
                writes[idx]
                    .clone()
                    .unwrap_or_else(|| panic!("No write to {name}{{{idx}}}"))
            }
            _ => port.clone(),
        }
    }
}

impl visitor::Transform for Lower {
    /// Mapping from component -> event -> max state
    type Info = HashMap<Id, HashMap<Id, u64>>;

    fn new(_: &core::Namespace, max_states: &Self::Info) -> Self {
        Self {
            fsms: HashMap::new(),
            max_states: max_states.clone(),
            bundle_writes: HashMap::new(),
        }
    }

    fn clear_data(&mut self) {
        self.fsms.clear();
        self.bundle_writes.clear();
    }

    /// Visit components with high-level invokes
    fn component_filter(&self, _: &CompBinding) -> bool {
        true
    }

    fn bundle(
        &mut self,
        bundle: core::Bundle,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        self.bundle_writes.insert(
            *bundle.name,
            vec![None; u64::try_from(bundle.len.inner()).unwrap() as usize],
        );
        Ok(vec![])
    }

    fn connect(
        &mut self,
        con: core::Connect,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        let src = self.port(con.src.inner());
        if let core::Port::Bundle { name, idx } = con.dst.inner() {
            let idx = u64::try_from(idx.inner()).unwrap() as usize;
            debug_assert!(
                self.bundle_writes[name.inner()][idx].is_none(),
                "multiple writes to {name}{{{idx}}}"
            );
            let writes = self.bundle_writes.get_mut(name).unwrap();
            writes[idx] = Some(src);
            // Remove assignment to bundle port
            Ok(vec![])
        } else {
            let con =
                core::Connect::new(con.dst, core::Loc::unknown(src), con.guard);
            Ok(vec![con.into()])
        }
    }

    // TODO(rachit): Document how the compilation works
    fn invoke(
        &mut self,
        inv: core::Invoke,
        ctx: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        // Compile only if this is a high-level invoke
        if let core::Invoke {
            name: bind,
            instance,
            abstract_vars,
            ports: Some(ports),
            ..
        } = inv
        {
            let idx = ctx.get_invoke_idx(&bind);
            let sig = idx.resolved_signature(ctx);
            // Get the signature associated with this instance.
            let binding = sig
                .event_binding(abstract_vars.iter().map(|t| t.inner().clone()));

            let mut connects = Vec::with_capacity(
                1 + ports.len() + sig.interface_signals.len(),
            );

            // Define the low-level invoke
            let low_inv =
                core::Invoke::new(bind.clone(), instance, abstract_vars, None)
                    .into();
            connects.push(low_inv);

            // Generate the assignment for each interface port
            for interface in &sig.interface_signals {
                let ev = &interface.event;
                // Get binding for this event in the invoke
                let t = binding.get(ev);
                let start_time = u64::try_from(t.offset()).unwrap();
                let port = self.get_fsm(&t.event()).port(start_time);
                let con = core::Connect::new(
                    Loc::unknown(core::Port::comp(
                        bind.clone(),
                        interface.name.clone(),
                    )),
                    Loc::unknown(port),
                    None,
                );
                connects.push(con.into())
            }

            // Generate assignment for each port
            for (src, formal) in ports.into_iter().zip(sig.inputs()) {
                let guard = self.range_to_guard(&formal.liveness);
                let port = self.port(src.inner());
                let con = core::Connect::new(
                    Loc::unknown(core::Port::comp(
                        bind.clone(),
                        formal.name.clone(),
                    )),
                    Loc::unknown(port),
                    guard,
                );
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
        ctx: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        let sig = ctx.this();

        // Define FSMs for each interface signal
        let events = &self.max_states[&sig.name];
        self.fsms = sig
            .interface_signals
            .iter()
            .map(|interface| {
                let ev = &interface.event;
                Ok((
                    *ev,
                    core::Fsm::new(
                        format!("{}_fsm", ev).into(),
                        events[ev],
                        core::Port::this(interface.name.clone()),
                    ),
                ))
            })
            .collect::<FilamentResult<HashMap<_, _>>>()?;

        Ok(vec![])
    }

    fn exit_component(
        &mut self,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        // Add the FSMs to the component
        let fsms = std::mem::take(&mut self.fsms)
            .into_values()
            .map(|f| f.into())
            .collect_vec();
        Ok(fsms)
    }
}
