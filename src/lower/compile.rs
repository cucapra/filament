use itertools::Itertools;

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
    fn get_fsm(&self, event: &ast::Id) -> &ast::Fsm {
        self.fsms
            .get(event)
            .unwrap_or_else(|| panic!("No FSM for event `{event}`."))
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    fn range_to_guard(&self, range: ast::Range) -> ast::Guard {
        if let Some((ev, st, end)) = range.as_offset() {
            (st..end)
                .into_iter()
                .map(|st| self.get_fsm(ev).port(st).into())
                .reduce(ast::Guard::or)
                .unwrap()
        } else {
            unimplemented!(
                "Range `{range}` cannot be represented as a simple non-max offset")
        }
    }

    fn max_state_from_sig(
        &mut self,
        sig: &ast::Signature,
        abstract_vars: &[ast::TimeRep],
        binding: &ast::Binding,
    ) {
        let out_events =
            sig.outputs.iter().chain(sig.inputs.iter()).flat_map(|pd| {
                pd.liveness
                    .resolve(binding)
                    .events()
                    .into_iter()
                    .cloned()
                    .collect_vec()
            });

        // Abstract variables can affect the max state calculation
        let abs_events = abstract_vars.iter().cloned();

        // Use all ranges to compute max state
        out_events.chain(abs_events).for_each(|fsm| {
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
        sig: &ast::Signature,
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
            // Get the signature associated with this instance.
            let binding = sig.binding(&abstract_vars)?;
            self.max_state_from_sig(sig, &abstract_vars, &binding);

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
                    Some(guard),
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
