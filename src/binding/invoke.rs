use super::{CompBinding, InstIdx, SigIdx};
use crate::core::{self, Id, Time, TimeSub};
use crate::utils::{self, GPosIdx};
use crate::{diagnostics, idx};
use itertools::Itertools;

/// Index to a bound invocation
pub type InvIdx = idx!(BoundInvoke);

impl InvIdx {
    /// Get the position of the invocation
    pub fn pos(&self, ctx: &CompBinding) -> GPosIdx {
        ctx[*self].pos
    }

    /// Get resolved event bindings for the invocation
    pub fn resolved_event_binding(&self, ctx: &CompBinding) -> Vec<Time> {
        let inv = &ctx[*self];
        let inst = &ctx[inv.instance];
        let param_b = ctx.prog[inst.sig].param_binding(&inst.params);

        inv.events
            .iter()
            .map(|e| e.clone().resolve_expr(&param_b))
            .collect()
    }

    /// Return the idx for the signature associated with the invocation.
    pub fn unresolved_signature(&self, ctx: &CompBinding) -> SigIdx {
        let inv = &ctx[*self];
        let inst = &ctx[inv.instance];
        inst.sig
    }

    /// Return the signature of the component being invoked using the parameter bindings and
    /// the event bindings of the invocation.
    pub fn resolved_signature(&self, ctx: &CompBinding) -> core::Signature {
        let inv = &ctx[*self];
        let inst_idx = inv.instance;
        let inst = &ctx[inst_idx];
        let event_b = ctx.prog[inst.sig].event_binding(inv.events.clone());
        inst_idx
            .param_resolved_signature(ctx)
            .resolve_event(&event_b)
    }

    /// Get the "active range" for an event in the invocation.
    /// If we have something like:
    /// ```
    /// comp Reg<G: L-(G+2), L: L-G>(...) { ... }
    ///
    /// comp main<T: 1> {
    ///   R := new Reg;
    ///   r0 := R<T+3, T+6>
    /// }
    /// ```
    ///
    /// The active ranges for events in the signature are:
    /// G -> [T+3, T+3+Delay(G)] = [T+3, T+4]
    /// L -> [T+5, T+5+Delay(L)] = [T+6, T+9]
    ///
    /// The function returns the (start_time, delay) for each event in the signature.
    pub fn event_active_ranges(
        &self,
        ctx: &CompBinding,
    ) -> Vec<(core::Loc<Time>, core::Loc<TimeSub>)> {
        let inv = &ctx[*self];
        let sig = self.resolved_signature(ctx);
        sig.events
            .iter()
            .zip(&inv.events)
            .map(|(ev, bind)| (bind.clone().into(), ev.delay.clone()))
            .collect_vec()
    }

    /// Fully resolve a port.
    /// Accepts a function to resolve the liveness of the port using time and width bindings.
    // XXX: Does not need to return an option
    pub fn get_invoke_port<F>(
        &self,
        ctx: &CompBinding,
        port: &Id,
        resolve_range: F,
    ) -> core::PortDef
    where
        F: Fn(
            &core::Range,
            &utils::Binding<Time>,
            &utils::Binding<core::Expr>,
        ) -> core::Range,
    {
        let inv = &ctx[*self];
        let inst = &ctx[inv.instance];
        let param_b = ctx.prog[inst.sig].param_binding(&inst.params);
        let event_b = ctx.prog[inst.sig].event_binding(inv.events.clone());
        let sig = &ctx.prog[inst.sig];
        let port = sig.get_port(port);
        core::PortDef::new(
            port.name.clone(),
            resolve_range(&port.liveness, &event_b, &param_b).into(),
            port.bitwidth.inner().clone().resolve(&param_b).into(),
        )
    }

    /// Get all the fully resolved constraints for the signature of an invocation.
    /// This includes:
    /// - The constraints of the component
    /// - Well-formedness constraints
    pub fn get_resolved_sig_constraints(
        &self,
        ctx: &CompBinding,
        diag: &mut diagnostics::Diagnostics,
    ) -> Vec<core::Loc<core::Constraint>> {
        let inv = &ctx[*self];
        let inst = &ctx[inv.instance];
        let sig_idx = inst.sig;
        let param_b = &ctx.prog[sig_idx].param_binding(&inst.params);
        let event_b = &ctx.prog[sig_idx].event_binding(inv.events.clone());
        let sig = &ctx.prog[sig_idx];
        sig.constraints
            .iter()
            .map(|c| {
                c.clone()
                    .map(|c| c.resolve_event(event_b).resolve_expr(param_b))
            })
            .chain(sig.well_formed(diag).into_iter().map(|c| {
                c.map(|c| c.resolve_event(event_b).resolve_expr(param_b))
            }))
            .collect()
    }
}

pub struct BoundInvoke {
    /// The instance being invoked
    pub instance: InstIdx,
    /// Event binding for this invocation
    pub events: Vec<Time>,
    /// Position associated with this invocation
    pub(super) pos: GPosIdx,
}

impl BoundInvoke {
    pub fn new(instance: InstIdx, events: Vec<Time>, pos: GPosIdx) -> Self {
        Self {
            instance,
            events,
            pos,
        }
    }
}
