use crate::errors::{FilamentResult, WithPos};
use crate::event_checker::ast;
use crate::visitor;
use std::collections::HashMap;

#[derive(Default)]
pub struct CompileInvokes {
    /// Mapping from events to FSMs
    fsms: HashMap<ast::Id, ast::Fsm>,
}

impl CompileInvokes {
    pub fn get_fsm(&self, event: &ast::Id) -> &ast::Fsm {
        self.fsms.get(event).unwrap_or_else(|| panic!("No FSM for event `{event}`. This likely happened because the input code already had an interface port defined for the event. The compiler assumes that for such code, the FSM is manually managed."))
    }

    /// Converts an interval to a guard expression with the appropriate FSM
    pub fn range_to_guard(&self, range: ast::Range) -> ast::Guard {
        if let Some((ev, st, end)) = range.as_offset() {
            (st..end)
                .into_iter()
                .map(|st| ast::Guard::Port(self.get_fsm(ev).port(st)))
                .reduce(|l, r| ast::Guard::Or(Box::new(l), Box::new(r)))
                .unwrap()
        } else {
            unimplemented!(
                "Range `{range}` cannot be represented as a simple non-max offset")
        }
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
            comp,
            abstract_vars,
            ports: Some(ports),
            ..
        } = inv
        {
            // Get the signature associated with this instance.
            let binding: HashMap<_, _> = sig
                .abstract_vars
                .iter()
                .cloned()
                .zip(abstract_vars.iter())
                .collect();

            let mut connects = Vec::with_capacity(
                1 + ports.len() + sig.interface_signals.len(),
            );

            // Define the low-level invoke
            let low_inv = ast::Invoke::new(
                bind.clone(),
                comp,
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
                let (_, start_time) =
                    binding[ev].as_unit().unwrap_or_else(|| {
                        unimplemented!(
                            "Binding for event {ev} is a max-expression"
                        )
                    });
                let port = self.get_fsm(ev).port(*start_time);
                let con = ast::Connect::new(
                    ast::Port::CompPort {
                        comp: bind.clone(),
                        name: interface.name.clone(),
                    },
                    port,
                    None,
                )
                .set_span(pos.clone());
                connects.push(con.into())
            }

            // Generate assignment for each port
            for ((port, _), formal) in ports.into_iter().zip(sig.inputs.iter())
            {
                let req = formal.liveness.resolve(&binding);
                assert!(
                    req.exact.is_none(),
                    "Cannot compile ports with exact specifications"
                );
                let guard = self.range_to_guard(req.within);
                let con = ast::Connect::new(
                    ast::Port::CompPort {
                        comp: bind.clone(),
                        name: formal.name.clone(),
                    },
                    port,
                    Some(guard),
                )
                .set_span(pos.clone());
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
        // Define FSMs for each missing port event
        self.fsms = comp
            .sig
            .interface_signals
            .iter()
            .map(|interface| {
                let ev = &interface.event;
                (
                    ev.clone(),
                    ast::Fsm::new(
                        format!("{}_fsm", ev).into(),
                        interface.delay(),
                        ast::Port::ThisPort(interface.name.clone()),
                    ),
                )
            })
            .collect::<HashMap<_, _>>();

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
            .map(|(_, fsm)| fsm.into())
            .chain(comp.body.into_iter())
            .collect();
        Ok(comp)
    }
}
