use itertools::Itertools;

use crate::errors::FilamentResult;
use crate::event_checker::ast;
use crate::visitor;
use std::collections::HashMap;

pub struct CompileInvokes<'a> {
    /// Signatures for instances
    pub sigs: HashMap<ast::Id, &'a ast::Signature>,

    /// Mapping from events to FSMs
    fsms: HashMap<ast::Id, ast::Fsm>,
}

impl CompileInvokes<'_> {
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

impl visitor::Transform for CompileInvokes<'_> {
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        sig: &ast::Signature,
    ) -> FilamentResult<Vec<ast::Command>> {
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
                comp.clone(),
                abstract_vars.clone(),
                None,
            )
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
                );
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
                );
                connects.push(con.into());
            }
            Ok(connects)
        } else {
            return Ok(vec![inv.into()]);
        }
    }
}

/// Computes the max state traversed by each event variable
fn max_states(
    comp: ast::Component,
    bindings: &visitor::Bindings,
) -> FilamentResult<ast::Component> {
    let ast::Component { mut sig, mut body } = comp;

    let defined_fsm_events = body
        .iter()
        .filter_map(|cmd| {
            if let ast::Command::Fsm(fsm) = cmd {
                Some(fsm.event)
            } else {
                None
            }
        })
        .collect_vec();

    // Define FSMs for each missing port event
    let fsms = missing_interfaces
        .iter()
        .map(|interface| {
            let ev = &interface.event;
            (
                ev.clone(),
                ast::Fsm::new(
                    format!("{}_fsm", ev).into(),
                    u64::MAX, // place holder value of the FSM. Patched up later.
                    ast::Port::ThisPort(interface.name.clone()),
                ),
            )
        })
        .collect::<HashMap<_, _>>();

    // Define max_state for each FSM to be 0.
    let max_states =
        sig.abstract_vars.iter().map(|ev| (ev.clone(), 0)).collect();

    let mut ctx = Context {
        fsms,
        max_states,
        ..Default::default()
    };

    // Compile the body
    let body = body
        .drain(..)
        .flat_map(|con| match con {
            ast::Command::Invoke(inv) => compile_invoke(inv, &mut ctx),
            ast::Command::Instance(ast::Instance {
                ref name,
                ref component,
            }) => {
                let sig = bindings.get(component);
                ctx.sigs.insert(name.clone(), sig);
                vec![con]
            }
            ast::Command::Connect(_) | ast::Command::Fsm(_) => vec![con],
        })
        .collect_vec();

    // Define the correct values for FSM states and add them to the body
    let body = ctx
        .fsms
        .into_iter()
        .map(|(ev, mut fsm)| {
            fsm.states = ctx.max_states[&ev];
            ast::Command::Fsm(fsm)
        })
        .chain(body)
        .collect_vec();

    // Fix up the interface signals delays for interfaces we added
    sig.interface_signals.iter_mut().for_each(|id| {
        if missing_events.contains(&id.event) {
            *id = ast::InterfaceDef::new(
                id.name.clone(),
                id.event.clone(),
                ctx.max_states[&id.event],
            )
        }
    });

    Ok(ast::Component { sig, body })
}

pub fn lower_invokes(mut ns: ast::Namespace) -> FilamentResult<ast::Namespace> {
    let comps = ns.components.drain(..).collect_vec();
    let ext_sigs = ns.signatures();
    let mut binding = Bindings {
        ext_sigs,
        ..Default::default()
    };

    for comp in comps {
        let new_comp = max_states(comp, &binding)?;
        binding.add_comp(new_comp);
    }

    ns.components = binding.into();

    Ok(ns)
}
