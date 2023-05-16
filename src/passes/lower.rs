use crate::ast::{self, Id, Loc};
use crate::binding::CompBinding;
use crate::errors::FilamentResult;
use crate::visitor;
use itertools::Itertools;
use std::collections::HashMap;

/// Compiles high-level invokes into low-level invokes by instantiating FSMs and generating guards.
/// Additionally removes all bundles and inlines their reads.
#[derive(Default)]
pub struct Lower {
    /// Mapping from events to FSMs
    fsms: HashMap<ast::Id, ast::Fsm>,
    /// Max state map
    max_states: HashMap<ast::Id, HashMap<ast::Id, u64>>,
    /// Track writes to bundles
    bundle_writes: HashMap<ast::Id, Vec<Option<ast::Port>>>,
}

impl Lower {
    fn find_fsm(&self, event: &ast::Id) -> Option<&ast::Fsm> {
        self.fsms.get(event)
    }

    fn get_fsm(&self, event: &ast::Id) -> &ast::Fsm {
        self.find_fsm(event)
            .unwrap_or_else(|| panic!("No FSM for event `{event}`."))
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    fn range_to_guard(&self, range: &ast::Range) -> Option<ast::Guard> {
        let Some((ev, st, end)) = range.as_offset() else {
            unreachable!(
                "Range `{range}` cannot be represented as a simple offset"
            )
        };

        let fsm = self.find_fsm(&ev)?;
        let guard = (st.try_into().unwrap()..end.try_into().unwrap())
            .map(|st| fsm.port(st).into())
            .reduce(ast::Guard::or)
            .unwrap();
        Some(guard)
    }

    fn port(&self, port: &ast::Port) -> ast::Port {
        match &port {
            ast::Port::Bundle { name, access } => {
                let ast::Access::Index(idx) = access.inner() else {
                    unreachable!("Unexpected bundle range access found: `{port}'")
                };
                let idx = u64::try_from(idx).unwrap() as usize;
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

    fn new(_: &ast::Namespace, max_states: &Self::Info) -> Self {
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
        bundle: ast::Bundle,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        self.bundle_writes.insert(
            *bundle.name,
            vec![None; u64::try_from(bundle.typ.len.inner()).unwrap() as usize],
        );
        Ok(vec![])
    }

    fn connect(
        &mut self,
        con: ast::Connect,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        let src = self.port(con.src.inner());
        if let ast::Port::Bundle { name, access } = con.dst.inner() {
            let ast::Access::Index(idx) = access.inner() else {
                unreachable!("Unexpected bundle range access found: `{}'", con.dst)
            };
            let idx = u64::try_from(idx).unwrap() as usize;
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
                ast::Connect::new(con.dst, ast::Loc::unknown(src), con.guard);
            Ok(vec![con.into()])
        }
    }

    // TODO(rachit): Document how the compilation works
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        ctx: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        // Compile only if this is a high-level invoke
        if let ast::Invoke {
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
                ast::Invoke::new(bind.clone(), instance, abstract_vars, None)
                    .into();
            connects.push(low_inv);

            // Generate the assignment for each interface port
            for interface in &sig.interface_signals {
                let ev = &interface.event;
                // Get binding for this event in the invoke
                let t = binding.get(ev);
                let start_time = u64::try_from(t.offset()).unwrap();
                let port = self.get_fsm(&t.event()).port(start_time);
                let con = ast::Connect::new(
                    Loc::unknown(ast::Port::inv_port(
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
                let guard = self.range_to_guard(formal.liveness());
                let port = self.port(src.inner());
                let con = ast::Connect::new(
                    Loc::unknown(ast::Port::inv_port(
                        bind.clone(),
                        formal.name().clone(),
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
    ) -> FilamentResult<Vec<ast::Command>> {
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
                    ast::Fsm::new(
                        format!("{}_fsm", ev).into(),
                        events[ev],
                        ast::Port::this(interface.name.clone()),
                    ),
                ))
            })
            .collect::<FilamentResult<HashMap<_, _>>>()?;

        Ok(vec![])
    }

    fn exit_component(
        &mut self,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        // Add the FSMs to the component
        let fsms = std::mem::take(&mut self.fsms)
            .into_values()
            .map(|f| f.into())
            .collect_vec();
        Ok(fsms)
    }
}
