use crate::core;
use crate::{errors::FilamentResult, event_checker::ast};
use std::collections::HashMap;

struct Context<'a> {
    /// Maximum state used by an event variable.
    pub max_states: HashMap<ast::Id, u64>,

    /// Signatures for instances
    pub sigs: HashMap<ast::Id, &'a ast::Signature>,

    /// Mapping from events to FSMs
    pub fsms: HashMap<ast::Id, &'a ast::Fsm>,
}

fn interval_to_guard(inv: ast::Range, ctx: &mut Context) -> ast::Guard {
    if let Some((ev, st, end)) = inv.as_offset() {
        // Update max state if this interval ends at a greater value
        if ctx.max_states[ev] < end {
            *ctx.max_states.get_mut(ev).unwrap() = end;
        }
        (st..end)
            .into_iter()
            .map(|st| ast::Guard::Port(ctx.fsms[ev].port(st)))
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

        let mut connects = Vec::with_capacity(ports.len());

        // For each port, generate the correct connect.
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
                        comp: comp.clone(),
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
    mut comp: ast::Component,
    ctx: &mut Context,
) -> FilamentResult<ast::Component> {
    let events = comp
        .sig
        .abstract_vars
        .iter()
        .map(|ev| (ev.clone(), 0))
        .collect();
    ctx.max_states = events;

    comp.body = comp
        .body
        .into_iter()
        .flat_map(|con| match con {
            ast::Command::Invoke(inv) => compile_invoke(inv, ctx),
            ast::Command::Instance(_) => vec![],
            ast::Command::Connect(_) => vec![],
            ast::Command::Fsm(_) => vec![],
        })
        .collect();

    Ok(comp)
}
