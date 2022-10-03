use itertools::Itertools;

use crate::core::WithTime;
use crate::errors::{FilamentResult, WithPos};
use crate::event_checker::ast;
use crate::visitor;
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileInvokes {
    /// Mapping from events to FSMs
    fsms: HashMap<ast::Id, ast::Fsm>,
    /// Mapping from event to their max state
    max_states: HashMap<ast::Id, u64>,
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
        if let Some((ev, st, end)) = range.as_offset() {
            let fsm = self.find_fsm(ev)?;
            let guard = (st..end)
                .into_iter()
                .map(|st| fsm.port(st).into())
                .reduce(ast::Guard::or)
                .unwrap();
            Some(guard)
        } else {
            unimplemented!(
                "Range `{range}` cannot be represented as a simple non-max offset")
        }
    }

    // When computing the max-state, anything that can act as an ouput matters.
    // For example, in
    //  in = g ? out;
    // We know that the interval for out is necessarily at least as big as in.
    // Therefore, in the max-state computation, we don't have to care about anything that is an
    // input.
    fn max_state_from_sig<W>(
        &mut self,
        resolved_outputs: impl Iterator<Item = ast::PortDef<W>>,
    ) where
        W: Clone,
    {
        let out_events = resolved_outputs.flat_map(|pd: ast::PortDef<W>| {
            pd.liveness.events().into_iter().cloned().collect_vec()
        });

        // Use all ranges to compute max state
        out_events.for_each(|fsm| {
            fsm.events().for_each(|(ev, &st)| {
                if self.max_states[ev] < st {
                    *self.max_states.get_mut(ev).unwrap() = st;
                }
            })
        });
    }
}

impl visitor::Transform for CompileInvokes {
    fn new(_: &ast::Namespace) -> Self {
        Self::default()
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
        sig: &visitor::ResolvedInstance,
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
            let sig = sig.resolve();
            // Get the signature associated with this instance.
            let binding = sig.binding(&abstract_vars);
            self.max_state_from_sig(
                sig.outputs.iter().map(|pd| pd.resolve(&binding)),
            );

            let mut connects = Vec::with_capacity(
                1 + ports.len() + sig.interface_signals.len(),
            );

            // Define the low-level invoke
            let low_inv = ast::Invoke::new(
                bind.clone(),
                instance,
                abstract_vars.clone(),
                None,
            )
            .set_span(pos.clone())
            .into();
            connects.push(low_inv);

            // Generate the assignment for each interface port
            for interface in &sig.interface_signals {
                // Skip phantom interfaces
                if interface.phantom {
                    continue;
                }
                let ev = &interface.event;
                // Get binding for this event in the invoke
                let (s_ev, start_time) =
                    binding.get(ev).as_unit().unwrap_or_else(|| {
                        unimplemented!(
                            "Binding for event {ev} is a max-expression"
                        )
                    });
                let port = self.get_fsm(s_ev).port(*start_time);
                let con = ast::Connect::new(
                    ast::Port::comp(bind.clone(), interface.name.clone()),
                    port,
                    None,
                )
                .set_span(pos.clone());
                connects.push(con.into())
            }

            // Generate assignment for each port
            for (port, formal) in ports.into_iter().zip(sig.inputs.iter()) {
                let req = formal.liveness.resolve(&binding);
                assert!(
                    req.exact.is_none(),
                    "Cannot compile ports with exact specifications"
                );
                let guard = self.range_to_guard(req.within);
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
        self.fsms = comp
            .sig
            .interface_signals
            .iter()
            .filter(|id| !id.phantom)
            .map(|interface| {
                let ev = &interface.event;
                Ok((
                    ev.clone(),
                    ast::Fsm::new(
                        format!("{}_fsm", ev).into(),
                        // Assign a fake number of states. We'll patch this up
                        // at the end.
                        u64::MAX,
                        ast::Port::this(interface.name.clone()),
                    ),
                ))
            })
            .collect::<FilamentResult<HashMap<_, _>>>()?;

        self.max_states = comp
            .sig
            .interface_signals
            .iter()
            .map(|ev| (ev.event.clone(), 0))
            .collect();

        // Inputs of the component act as outputs inside the body
        self.max_state_from_sig(comp.sig.inputs.iter().cloned());

        Ok(comp)
    }

    fn exit_component(
        &mut self,
        mut comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        // Add all the FSMs
        comp.body = self
            .fsms
            .drain()
            .map(|(ev, fsm)| {
                ast::Fsm::new(fsm.name, self.max_states[&ev], fsm.trigger)
                    .into()
            })
            .chain(comp.body.into_iter())
            .collect();
        self.max_states = HashMap::default();
        Ok(comp)
    }
}
