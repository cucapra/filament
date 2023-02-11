use super::{FilSolver, IntervalCheck};
use crate::core::{self, OrderConstraint, TimeRep, WidthRep, WithTime};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor::{self, Checker, CompBinding};
use std::iter;

impl<T: TimeRep<Offset = W>, W: WidthRep> visitor::Checker<T, W>
    for IntervalCheck<T, W>
{
    fn new(ns: &core::Namespace<T, W>) -> FilamentResult<Self> {
        let mut solver = FilSolver::new()?;

        // Check that all signatures are well formed
        let t = std::time::Instant::now();
        for (_, sig) in ns.signatures() {
            log::trace!("===== Signature {} =====", &sig.name);
            solver.prove(
                sig.events().chain(sig.params.iter().cloned()),
                sig.constraints.clone(),
                sig.well_formed(),
                vec![],
            )?;
            log::trace!("==========");
        }
        log::info!("interval-check.sigs: {}ms", t.elapsed().as_millis());

        Ok(Self::from(solver))
    }

    fn clear_data(&mut self) {
        self.obligations.clear();
        self.facts.clear();
    }

    fn connect(
        &mut self,
        con: &core::Connect,
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<()> {
        let src = &con.src;
        let dst = &con.dst;
        log::trace!("Checking connect: {} = {}", dst, src);

        let resolve_range =
            |r: &core::Range<T>,
             event_b: &core::Binding<T>,
             param_b: &core::Binding<W>| {
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
                    .add_note(
                        format!(
                            "Source is available for {}",
                            guarantee.liveness
                        ),
                        src_pos,
                    )
                    .add_note(
                        format!("Destination's requirement {}", requirement),
                        dst.copy_span(),
                    )
            });
            self.add_obligations(within_fact);
        }

        Ok(())
    }

    fn invoke(
        &mut self,
        invoke: &core::Invoke<T>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        // Check the bindings for abstract variables do not violate @interface
        // requirements
        self.check_invoke_binds(invoke, ctx)?;

        // Check that the invocation's port connects are valid
        self.check_invoke_ports(invoke, ctx)?;

        // Check that the invocation's events satisfy well-formedness the component's constraints
        let constraints = ctx
            .get_invoke_idx(&invoke.name)
            .unwrap()
            .get_resolved_sig_constraints(ctx, |c, e, p| {
                c.resolve_event(e).resolve_offset(p)
            });

        constraints.into_iter().for_each(|con| {
            self.add_obligations(iter::once(con).map(|e| {
                e.add_note(
                    "Component's where clause constraints must be satisfied",
                    invoke.copy_span(),
                )
            }))
        });

        Ok(())
    }

    fn enter_component(
        &mut self,
        comp: &core::Component<T, W>,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        // Ensure that the signature is well-formed
        self.add_obligations(comp.sig.well_formed());

        // User-level components are not allowed to have ordering constraints. See https://github.com/cucapra/filament/issues/27.
        for constraint in &comp.sig.constraints {
            if constraint.is_ordering() {
                return Err(Error::malformed(
                    "User-level components cannot have ordering constraints",
                )
                .add_note(
                    format!("Component defines the constraint {constraint}"),
                    GPosIdx::UNKNOWN,
                ));
            } else {
                self.add_fact(constraint.clone())
            }
        }

        Ok(())
    }

    fn exit_component(
        &mut self,
        comp: &core::Component<T, W>,
        ctx: &CompBinding<T, W>,
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
            obs.into_iter(),
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

impl<T: TimeRep<Offset = W>, W: WidthRep> IntervalCheck<T, W> {
    /// Check that the events provided to an invoke obey the constraints implied
    /// by the component's delays.
    fn check_invoke_binds(
        &mut self,
        invoke: &core::Invoke<T>,
        ctx: &CompBinding<T, W>,
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
                .add_note(
                    "Event provided to invoke triggers too often",
                    invoke.copy_span(),
                )
                .add_note(
                    format!(
                        "Provided event may trigger every {} cycles",
                        ev_delay,
                    ),
                    ev.copy_span(),
                )
                .add_note(
                    format!(
                        "Interface requires event to trigger once in {} cycles",
                        this_ev_delay,
                    ),
                    this_ev.copy_span(),
                );
            constraints.push(cons);
        }
        self.add_obligations(constraints.into_iter());

        Ok(())
    }

    fn check_invoke_ports(
        &mut self,
        invoke: &core::Invoke<T>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        // If this is a high-level invoke, check all port requirements
        if let Some(actuals) = invoke.ports.clone() {
            let sig = ctx.get_invoke_sig(&invoke.name);
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
