use crate::ast::param as ast;
use crate::core::{self, Id, WithTime};
use crate::errors::{FilamentResult, WithPos};
use crate::visitor;
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileInvokes {
    /// Mapping from events to FSMs
    fsms: HashMap<ast::Id, ast::Fsm>,
    /// Max state map
    max_states: HashMap<ast::Id, HashMap<ast::Id, u64>>,
}

impl CompileInvokes {
    fn find_fsm(&self, event: &ast::Id) -> Option<&ast::Fsm> {
        self.fsms.get(event)
    }

    fn get_fsm(&self, event: &ast::Id) -> &ast::Fsm {
        self.find_fsm(event)
            .unwrap_or_else(|| panic!("No FSM for event `{event}`."))
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    fn range_to_guard(&self, range: ast::Range) -> Option<ast::Guard> {
        let Some((ev, st, end)) = range.as_offset() else {
            unreachable!(
                "Range `{range}` cannot be represented as a simple non-max offset"
            )
        };

        let fsm = self.find_fsm(&ev)?;
        let guard = (st..end)
            .into_iter()
            .map(|st| fsm.port(st).into())
            .reduce(ast::Guard::or)
            .unwrap();
        Some(guard)
    }
}

impl visitor::Transform<core::Time<u64>, core::PortParam> for CompileInvokes {
    /// Mapping from component -> event -> max state
    type Info = HashMap<Id, HashMap<Id, u64>>;

    fn new(_: &ast::Namespace, max_states: &Self::Info) -> Self {
        Self {
            fsms: HashMap::new(),
            max_states: max_states.clone(),
        }
    }

    fn clear_data(&mut self) {
        self.fsms.clear();
    }

    /// Visit components with high-level invokes
    fn component_filter(&self, comp: &ast::Component) -> bool {
        comp.body.iter().any(|con| {
            if let ast::Command::Invoke(ast::Invoke { ports, .. }) = con {
                ports.is_some()
            } else {
                false
            }
        })
    }

    // TODO(rachit): Document how the compilation works
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        sig: &ast::ResolvedInstance,
    ) -> FilamentResult<Vec<ast::Command>> {
        let pos = inv.copy_span();
        // Compile only if this is a high-level invoke
        if let ast::Invoke {
            bind,
            instance,
            abstract_vars,
            ports: Some(ports),
            ..
        } = inv
        {
            let sig = sig.resolve()?;
            // Get the signature associated with this instance.
            let binding = sig.binding(&abstract_vars);

            let mut connects = Vec::with_capacity(
                1 + ports.len() + sig.interface_signals.len(),
            );

            // Define the low-level invoke
            let low_inv =
                ast::Invoke::new(bind.clone(), instance, abstract_vars, None)
                    .set_span(pos)
                    .into();
            connects.push(low_inv);

            // Generate the assignment for each interface port
            for interface in &sig.interface_signals {
                let ev = &interface.event;
                // Get binding for this event in the invoke
                let t = binding.get(ev);
                let start_time = t.offset();
                let port = self.get_fsm(&t.event).port(start_time);
                let con = ast::Connect::new(
                    ast::Port::comp(bind.clone(), interface.name.clone()),
                    port,
                    None,
                )
                .set_span(pos);
                connects.push(con.into())
            }

            // Generate assignment for each port
            for (port, formal) in ports.into_iter().zip(sig.inputs()) {
                let req = formal.liveness.resolve(&binding);
                let guard = self.range_to_guard(req);
                let sp = port.copy_span();
                let con = ast::Connect::new(
                    ast::Port::comp(bind.clone(), formal.name.clone()),
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
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
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
                    ast::Fsm::new(
                        format!("{}_fsm", ev).into(),
                        events[ev],
                        ast::Port::this(interface.name.clone()),
                    ),
                ))
            })
            .collect::<FilamentResult<HashMap<_, _>>>()?;

        Ok(comp)
    }

    fn exit_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        // Add the FSMs to the component
        let mut comp = comp;
        let fsms = std::mem::take(&mut self.fsms)
            .into_values()
            .map(|f| f.into());
        comp.body = fsms.chain(comp.body.into_iter()).collect();
        Ok(comp)
    }
}
