use crate::{
    binding, cmdline,
    core::{self, Id},
    visitor::{self, Traverse},
};
use std::collections::HashMap;

type States = HashMap<Id, u64>;

/// Compute the maximum number of states for each event in a component
#[derive(Default)]
pub struct MaxStates {
    /// Map for each event to the maximum number of states for each component
    pub max_states: HashMap<Id, States>,
    /// Current set of states we're working on
    cur_states: States,
    diag: crate::diagnostics::Diagnostics,
}

impl MaxStates {
    fn max_state_from_ports<'a>(
        &mut self,
        ports: impl IntoIterator<Item = &'a core::Loc<core::PortDef>>,
    ) {
        for pd in ports {
            for time in pd.inner().liveness().time_exprs() {
                let ev = &time.event;
                let v = self.cur_states.get_mut(ev).unwrap();
                let st = u64::try_from(time.offset()).unwrap();
                if *v < st {
                    *v = st;
                }
            }
        }
    }
}

impl visitor::Checker for MaxStates {
    fn new(_opts: &cmdline::Opts, _: &core::Namespace) -> Self {
        Self::default()
    }
    fn clear_data(&mut self) {
        self.cur_states.clear();
    }
    fn diagnostics(&mut self) -> &mut crate::diagnostics::Diagnostics {
        &mut self.diag
    }
    fn enter_component(
        &mut self,
        comp: &core::Component,
        _: &binding::CompBinding,
    ) -> Traverse {
        self.cur_states = comp
            .sig
            .events
            .iter()
            .map(|eb| (*eb.event.inner(), 0))
            .collect();
        self.max_state_from_ports(comp.sig.inputs());
        Traverse::Continue(())
    }
    fn invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        // Get the fully resolved signature
        let inv_idx = ctx.get_invoke_idx(&inv.name);
        let sig = inv_idx.resolved_signature(ctx);
        self.max_state_from_ports(sig.outputs());
        Traverse::Continue(())
    }
    fn exit_component(
        &mut self,
        comp: &core::Component,
        _: &binding::CompBinding,
    ) -> Traverse {
        let events = std::mem::take(&mut self.cur_states);
        log::info!("Max states for {}: {:?}", comp.sig.name, events);
        self.max_states.insert(*comp.sig.name.inner(), events);
        Traverse::Continue(())
    }
}
