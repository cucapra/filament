use super::{CompBinding, InstIdx, SigIdx};
use crate::core::{self, Id, Time, TimeSub};
use crate::diagnostics;
use crate::errors::WithPos;
use crate::utils::{self, GPosIdx};
use itertools::Itertools;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an invocation bound in a component.
/// Defined methods represent operations on an invocation and require a
/// component binding to be resolved.
pub struct InvIdx(pub(super) usize);

impl InvIdx {
    /// Get the position of the invocation
    pub fn pos(&self, ctx: &CompBinding) -> GPosIdx {
        ctx[*self].pos
    }

    /// Get resolved event bindings for the invocation
    pub fn resolved_event_binding(&self, ctx: &CompBinding) -> Vec<Time> {
        let inv = &ctx[*self];
        let inst = &ctx[inv.instance];
        let param_b = ctx.prog.param_binding(inst.sig, &inst.params);

        inv.events
            .iter()
            .map(|e| e.resolve_expr(&param_b))
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
        let event_b = ctx.prog.event_binding(inst.sig, &inv.events);
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
    ) -> Vec<(Time, TimeSub)> {
        let inv = &ctx[*self];
        let sig = self.resolved_signature(ctx);
        sig.events
            .iter()
            .zip(&inv.events)
            .map(|(ev, bind)| (bind.clone(), ev.delay.clone()))
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
        let param_b = ctx.prog.param_binding(inst.sig, &inst.params);
        let event_b = ctx.prog.event_binding(inst.sig, &inv.events);
        let sig = ctx.prog.sig(inst.sig);
        let port = sig.get_port(port);
        core::PortDef::new(
            port.name.clone(),
            resolve_range(&port.liveness, &event_b, &param_b),
            port.bitwidth.resolve(&param_b),
        )
    }

    /// Get all the fully resolved constraints for the signature of an invocation.
    /// This includes:
    /// - The constraints of the component
    /// - Well-formedness constraints
    pub fn get_resolved_sig_constraints<F>(
        &self,
        ctx: &CompBinding,
        resolve_constraint: F,
        diag: &mut diagnostics::Diagnostics,
    ) -> Vec<core::Constraint>
    where
        F: Fn(
            &core::Constraint,
            &utils::Binding<Time>,
            &utils::Binding<core::Expr>,
        ) -> core::Constraint,
    {
        let inv = &ctx[*self];
        let inst = &ctx[inv.instance];
        let sig_idx = inst.sig;
        let param_b = &ctx.prog.param_binding(sig_idx, &inst.params);
        let event_b = &ctx.prog.event_binding(sig_idx, &inv.events);
        let resolve_ref = |c| resolve_constraint(c, event_b, param_b);
        let resolve = |c| resolve_constraint(&c, event_b, param_b);
        let sig = ctx.prog.sig(sig_idx);
        sig.constraints
            .iter()
            .map(resolve_ref)
            .chain(sig.well_formed(diag).into_iter().map(resolve))
            .collect()
    }
}

pub struct BoundInvoke {
    /// The instance being invoked
    pub instance: InstIdx,
    /// Event binding for this invocation
    pub events: Vec<Time>,
    /// Position associated with this invocation
    pos: GPosIdx,
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

impl WithPos for BoundInvoke {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}
