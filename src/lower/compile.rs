use std::collections::HashMap;

use crate::event_checker::ast;

struct Context {
    /// Maximum state used by an event variable.
    pub max_states: HashMap<ast::Id, u64>,
}

/// Computes the max state traversed by each event variable
fn max_states(comp: &ast::Component, ctx: &mut Context) {
    let events = comp
        .sig
        .abstract_vars
        .iter()
        .map(|ev| (ev.clone(), 0))
        .collect();
    ctx.max_states = events;

    for con in &comp.body {
        match con {
            crate::core::Command::Invoke(_) => todo!(),
            crate::core::Command::Instance(_) => todo!(),
            crate::core::Command::Connect(_) => todo!(),
            crate::core::Command::Fsm(_) => todo!(),
        }
    }
}

fn interval_to_guard(inv: ast::Interval) -> ast::Guard {
    todo!()
}
