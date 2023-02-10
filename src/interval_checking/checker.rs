use super::{FilSolver, IntervalCheck};
use crate::core::{self, OrderConstraint, TimeRep, WidthRep, WithTime};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor::{self, Checker, CompBinding};
use std::iter;

impl<T: TimeRep<Offset = W>, W: WidthRep> visitor::Checker<T, W>
    for IntervalCheck<T>
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
        self.event_binds.clear();
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

        let resolve_liveness =
            |r: &core::Range<T>,
             event_b: &core::Binding<T>,
             param_b: &core::Binding<W>| {
                r.resolve_event(event_b).resolve_offset(param_b)
            };

        let requirement = ctx
            .get_resolved_port(dst, resolve_liveness)
            .unwrap()
            .liveness;
        let guarantee = ctx.get_resolved_port(src, resolve_liveness);
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
            .get_resolved_sig_constraints(&invoke.name, |c, e, p| {
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
        let (disj, share) = self.drain_sharing(ctx)?;

        // Prove all the required obligations
        let obs = self.drain_obligations();
        let t = std::time::Instant::now();
        self.solver.prove(
            comp.sig.events().chain(comp.sig.params.iter().cloned()),
            // XXX(rachit): Unnecessary clone
            self.facts.clone(),
            obs.into_iter().chain(disj.into_iter()),
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

impl<T: TimeRep<Offset = W>, W: WidthRep> IntervalCheck<T> {
    /// Check that the events provided to an invoke obey the constraints implied
    /// by the component's delays.
    fn check_invoke_binds(
        &mut self,
        invoke: &core::Invoke<T>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        let sig = ctx.get_invoke_sig(&invoke.name);
        let binding = ctx.prog.event_binding(sig, &invoke.abstract_vars);

        // Track event bindings for generating delay constraints
        self.add_event_binds(
            invoke.instance.clone(),
            &binding,
            invoke.copy_span(),
        );

        let mut constraints = vec![];

        // For each event provided in the bining, ensure that the corresponding interface
        // does not pulse more often than the interface allows.
        for (abs, evs) in binding.iter() {
            let inst_event = ctx.prog.get_event(sig, abs);

            // Each event in the binding must pulse less often than the interface of the abstract
            // variable.
            let event = &evs.event();
            // Get interface for this event
            let event_interface = ctx.prog.get_event(ctx.sig(), event);
            let int_len = inst_event.delay.resolve_event(&binding);
            let ev_int_len = &event_interface.delay;

            // Generate constraint
            let cons = core::Constraint::sub(core::OrderConstraint::gte(
                ev_int_len.clone(),
                int_len.clone(),
            ))
            .add_note(
                "Event provided to invoke pulses more often than event allows",
                invoke.copy_span(),
            )
            .add_note(
                format!(
                    "Provided event may trigger every {} cycles",
                    ev_int_len,
                ),
                event_interface.copy_span(),
            )
            .add_note(
                format!(
                    "Delay requires event to trigger once in {} cycles",
                    int_len,
                ),
                inst_event.copy_span(),
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
