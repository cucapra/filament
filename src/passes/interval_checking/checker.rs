use itertools::Itertools;

use super::IntervalCheck;
use crate::core::{self, Expr, OrderConstraint, Time};
use crate::diagnostics;
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::{self, FilSolver, GPosIdx};
use crate::visitor::{self, Checker, CompBinding};
use std::iter;

impl visitor::Checker for IntervalCheck {
    fn new(ns: &core::Namespace) -> FilamentResult<Self> {
        let mut solver = FilSolver::new()?;
        let mut diagnostics = diagnostics::Diagnostics::default();

        // Check that all signatures are well formed
        let t = std::time::Instant::now();
        for (_, sig) in ns.signatures() {
            log::trace!("===== Signature {} =====", &sig.name);
            solver.prove(
                sig.events().chain(sig.params.iter().cloned()),
                sig.constraints.clone(),
                sig.well_formed(&mut diagnostics),
                vec![],
            )?;
            log::trace!("==========");
        }
        log::info!("interval-check.sigs: {}ms", t.elapsed().as_millis());

        Ok(Self::from((solver, diagnostics)))
    }

    fn clear_data(&mut self) {
        self.obligations.clear();
        self.facts.clear();
    }

    fn connect(
        &mut self,
        con: &core::Connect,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        let src = &con.src;
        let dst = &con.dst;
        log::trace!("Checking connect: {} = {}", dst, src);

        let resolve_range =
            |r: &core::Range,
             event_b: &utils::Binding<Time>,
             param_b: &utils::Binding<Expr>| {
                r.resolve_offset(param_b).resolve_event(event_b)
            };

        let requirement =
            ctx.get_resolved_port(dst, resolve_range).unwrap().liveness;
        let guarantee = ctx.get_resolved_port(src, resolve_range);
        let src_pos = src.copy_span();

        // If we have: dst = src. We need:
        // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
        // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
        if let Some(guarantee) = &guarantee {
            let within_fact = OrderConstraint::subset(
                requirement.clone(),
                guarantee.liveness.clone(),
            )
            .map(|e| {
                core::Constraint::base(e)
                    .add_note(self.diag.add_info(
                        format!(
                            "Source is available for {}",
                            guarantee.liveness
                        ),
                        src_pos,
                    ))
                    .add_note(self.diag.add_info(
                        format!("Destination's requirement {}", requirement),
                        dst.copy_span(),
                    ))
            })
            .collect_vec();
            self.add_obligations(within_fact);
        }

        Ok(())
    }

    fn invoke(
        &mut self,
        invoke: &core::Invoke,
        ctx: &CompBinding,
    ) -> FilamentResult<()> {
        // Check the bindings for abstract variables do not violate @interface
        // requirements
        self.check_invoke_binds(invoke, ctx)?;

        // Check that the invocation's port connects are valid
        self.check_invoke_ports(invoke, ctx)?;

        // Check that the invocation's events satisfy well-formedness the component's constraints
        let sig = ctx
            .get_invoke_idx(&invoke.name)
            .unwrap()
            .resolved_signature(ctx);

        let constraints = sig
            .constraints
            .iter()
            .cloned()
            .chain(sig.well_formed(&mut self.diag).into_iter());

        for con in constraints {
            let con_with_info = con.add_note(self.diag.add_info(
                "Component's where clause constraints must be satisfied",
                invoke.copy_span(),
            ));
            self.add_obligations(iter::once(con_with_info));
        }

        Ok(())
    }

    fn enter_component(
        &mut self,
        comp: &core::Component,
        _: &CompBinding,
    ) -> FilamentResult<()> {
        // Ensure that the signature is well-formed
        let cons = comp.sig.well_formed(&mut self.diag);
        self.add_obligations(cons);

        // User-level components are not allowed to have ordering constraints. See https://github.com/cucapra/filament/issues/27.
        for constraint in &comp.sig.constraints {
            if constraint.is_ordering() {
                return Err(Error::malformed(
                    "User-level components cannot have ordering constraints",
                )
                .add_note(self.diag.add_info(
                    format!("Component defines the constraint {constraint}"),
                    GPosIdx::UNKNOWN,
                )));
            } else {
                self.add_fact(constraint.clone())
            }
        }

        Ok(())
    }

    fn exit_component(
        &mut self,
        comp: &core::Component,
        ctx: &CompBinding,
    ) -> FilamentResult<()> {
        // Add obligations from disjointness constraints
        let share = self.drain_sharing(ctx)?;

        // Prove all the required obligations
        let obs = self.drain_obligations();
        let t = std::time::Instant::now();
        self.solver.prove(
            comp.sig.events().chain(comp.sig.params.iter().cloned()),
            // XXX(rachit): Unnecessary clone
            self.facts.clone(),
            obs,
            share,
        )?;
        log::info!(
            "interval-check.{}.prove: {}ms",
            comp.sig.name,
            t.elapsed().as_millis()
        );

        Ok(())
    }
}

impl IntervalCheck {
    /// Check that the events provided to an invoke obey the constraints implied
    /// by the component's delays.
    fn check_invoke_binds(
        &mut self,
        invoke: &core::Invoke,
        ctx: &CompBinding,
    ) -> FilamentResult<()> {
        let inv_sig = ctx
            .get_invoke_idx(&invoke.name)
            .unwrap()
            .resolved_signature(ctx);
        let binds = &ctx.get_invoke(&invoke.name).events;
        let this_sig = ctx.prog.comp_sig(ctx.sig());

        let mut constraints = vec![];

        // For each event provided in the bindings, ensure that the delay is
        // less than the delay in the interface.
        for (idx, ev_expr) in binds.iter().enumerate() {
            let ev = ev_expr.event();

            let this_ev = this_sig.get_event(&ev);
            let this_ev_delay = &this_ev.delay;
            let ev = &inv_sig.events[idx];
            let ev_delay = &ev.delay;

            // Generate constraint
            let cons =
                core::Constraint::sub(core::OrderConstraint::gte(
                    this_ev_delay.clone(),
                    ev_delay.clone(),
                ))
                .add_note(self.diag.add_info(
                    "Event provided to invoke triggers too often",
                    invoke.copy_span(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "Provided event may trigger every {} cycles",
                        ev_delay,
                    ),
                    ev.copy_span(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "Interface requires event to trigger once in {} cycles",
                        this_ev_delay,
                    ),
                    this_ev.copy_span(),
                ));
            constraints.push(cons);
        }
        self.add_obligations(constraints.into_iter());

        Ok(())
    }

    fn check_invoke_ports(
        &mut self,
        invoke: &core::Invoke,
        ctx: &CompBinding,
    ) -> FilamentResult<()> {
        // If this is a high-level invoke, check all port requirements
        if let Some(actuals) = invoke.ports.clone() {
            let inv_idx = ctx.get_invoke_idx(&invoke.name).unwrap();
            // We use an unresolved signature here because [[Self::connect]] will eventually resolve them using
            // [[CompBinding::get_resolved_ports]
            let sig = inv_idx.unresolved_signature(ctx);
            let inputs = ctx.prog.input_names(sig);
            // Check connections implied by the invocation
            for (actual, formal) in actuals.iter().zip(inputs) {
                let dst = core::Port::comp(invoke.name.clone(), formal.clone())
                    .set_span(formal.copy_span());
                let con = core::Connect::new(dst, actual.clone(), None);
                self.connect(&con, ctx)?;
            }
        }

        Ok(())
    }
}
