use itertools::Itertools;

use crate::{errors::FilamentResult, event_checker::ast};
use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

#[derive(Default)]
struct Context<'a> {
    /// Signatures for instances
    pub sigs: HashMap<ast::Id, &'a ast::Signature>,

    /// Maximum state used by an event variable.
    pub max_states: HashMap<ast::Id, u64>,

    /// Mapping from events to FSMs
    pub fsms: HashMap<ast::Id, ast::Fsm>,
}

/// Converts an interval to a guard expression with the appropriate FSM
fn interval_to_guard(inv: ast::Range, ctx: &mut Context) -> ast::Guard {
    if let Some((ev, st, end)) = inv.as_offset() {
        // Update max state if this interval ends at a greater value
        if ctx.max_states[ev] < end {
            *ctx.max_states.get_mut(ev).unwrap() = end;
        }
        (st..end)
            .into_iter()
            .map(|st| {
                ast::Guard::Port(
                    ctx.fsms
                        .get(ev)
                        .unwrap_or_else(|| panic!("FSM {ev} missing"))
                        .port(st),
                )
            })
            .reduce(|l, r| ast::Guard::Or(Box::new(l), Box::new(r)))
            .unwrap()
    } else {
        panic!("Cannot compile ranges that are not simple offsets ")
    }
}

fn compile_invoke(inv: ast::Invoke, ctx: &mut Context) -> Vec<ast::Command> {
    if let ast::Invoke {
        bind,
        comp,
        abstract_vars,
        ports: Some(ports),
        ..
    } = inv
    {
        // Get the signature associated with this instance.
        let sig = ctx.sigs[&comp];
        let binding: HashMap<_, _> = sig
            .abstract_vars
            .iter()
            .cloned()
            .zip(abstract_vars.iter())
            .collect();

        let mut connects =
            Vec::with_capacity(1 + ports.len() + sig.interface_signals.len());

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
            let (_, start_time) = binding[ev].as_unit().unwrap_or_else(|| {
                unimplemented!("Binding for event {ev} is a max-expression")
            });
            let port = ctx.fsms[ev].port(*start_time);
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

        // Outputs can affect the max state calculation
        abstract_vars.iter().for_each(|fsm| {
            fsm.events().for_each(|(ev, &st)| {
                if ctx
                    .max_states
                    .get(ev)
                    .unwrap_or_else(|| panic!("Missing max state for {ev}"))
                    < &st
                {
                    *ctx.max_states.get_mut(ev).unwrap() = st;
                }
            })
        });

        // Generate assignment for each port
        for (port, formal) in ports.into_iter().zip(sig.inputs.iter()) {
            if let Some(live) = &formal.liveness {
                let req = live.resolve(&binding);
                assert!(
                    req.exact.is_none(),
                    "Cannot compile ports with exact specifications"
                );
                let guard = interval_to_guard(req.within, ctx);
                let con = ast::Connect::new(
                    ast::Port::CompPort {
                        comp: bind.clone(),
                        name: formal.name.clone(),
                    },
                    port,
                    Some(guard),
                );
                connects.push(con.into());
            } else {
                panic!("Unannotated ports cannot be compiled")
            }
        }
        connects
    } else {
        return vec![inv.into()];
    }
}

/// Computes the max state traversed by each event variable
fn max_states(
    comp: ast::Component,
    comp_sigs: &HashMap<ast::Id, &ast::Signature>,
) -> FilamentResult<ast::Component> {
    let ast::Component { sig, mut body } = comp;

    // Extend the signature to add missing interface signals for the defined parameters.
    let mut sig = Rc::try_unwrap(sig).unwrap();
    let defined_interfaces = sig
        .interface_signals
        .iter()
        .map(|id| id.name.clone())
        .collect();
    let all_events = sig.abstract_vars.iter().cloned().collect::<HashSet<_>>();

    sig.interface_signals.extend(
        all_events
            .difference(&defined_interfaces)
            .into_iter()
            .map(|ev| {
                ast::InterfaceDef::new(
                    format!("go_{}", ev).into(),
                    ev.clone(),
                    u64::MAX,
                )
            }),
    );

    // Define max_state for each FSM to be 0.
    let max_states =
        sig.abstract_vars.iter().map(|ev| (ev.clone(), 0)).collect();

    let mut ctx = Context {
        max_states,
        ..Default::default()
    };

    // Define FSMs for each event
    ctx.fsms = sig
        .interface_signals
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

    // Compile the body
    let body = body
        .drain(..)
        .flat_map(|con| match con {
            ast::Command::Invoke(inv) => compile_invoke(inv, &mut ctx),
            ast::Command::Instance(ast::Instance {
                ref name,
                ref component,
            }) => {
                let sig = comp_sigs[component];
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

    // Fix up the interface signals delays
    sig.interface_signals
        .iter_mut()
        .for_each(|mut id| id.delay = ctx.max_states[&id.event]);

    Ok(ast::Component {
        sig: Rc::new(sig),
        body,
    })
}

pub fn lower_invokes(mut ns: ast::Namespace) -> FilamentResult<ast::Namespace> {
    let sigs = ns
        .externs
        .iter()
        .flat_map(|(_, comps)| comps.iter().map(|s| (s.name.clone(), s)))
        .collect::<HashMap<_, _>>();

    ns.components = ns
        .components
        .into_iter()
        .map(|comp| max_states(comp, &sigs))
        .collect::<FilamentResult<_>>()?;

    println!("{}", ns);

    Ok(ns)
}
