use super::IntervalCheck;
use crate::binding::CompBinding;
use crate::core::{self, ForLoop, OrderConstraint, Time};
use crate::diagnostics;
use crate::errors::{Error, WithPos};
use crate::utils::{self, FilSolver};
use crate::visitor::{self, Checker, Traverse};
use itertools::Itertools;
use std::iter;

impl IntervalCheck {
    /// Check that a bundle access is within bounds
    fn port_bundle_index(&mut self, port: &core::Port, ctx: &CompBinding) {
        let core::PortType::Bundle { name, idx } = &port.typ else {
            return
        };
        let bun_idx = ctx.get_bundle_idx(name);
        let bun_len = &ctx[bun_idx].len;
        let greater = core::Constraint::sub(OrderConstraint::lt(
            idx.clone().into(),
            bun_len.clone().into(),
        ))
        .add_note(
            self.diag
                .add_info(
                    format!("cannot prove within-bounds bundle access: index {idx} greater than bundle length {bun_len}"),
                    port.copy_span()
                ),
        );
        let smaller = core::Constraint::sub(OrderConstraint::gte(
            idx.clone().into(),
            core::Expr::from(0).into(),
        )).add_note(
            self.diag
                .add_info(
                    format!("cannot prove within-bounds bundle access: index {idx} less than 0"),
                    port.copy_span()
                ),
        );
        self.add_obligations([greater, smaller]);
    }

    fn check_width(
        &mut self,
        con: &core::Connect,
        // Resolved ports
        src: &Option<core::PortDef>,
        dst: &Option<core::PortDef>,
    ) {
        let dst_w = dst
            .as_ref()
            .map(|p| p.bitwidth.clone())
            .unwrap_or_else(|| 32.into());
        let src_w = src
            .as_ref()
            .map(|p| p.bitwidth.clone())
            .unwrap_or_else(|| 32.into());

        // If we can't syntactically check
        let cons = core::Constraint::sub(core::OrderConstraint::eq(
            dst_w.clone().into(),
            src_w.clone().into(),
        ))
        .add_note(self.diag.add_info(
            format!("source `{}' has width {src_w}", con.src.name()),
            con.src.copy_span(),
        ))
        .add_note(self.diag.add_info(
            format!("destination `{}' has width {dst_w}", con.dst.name(),),
            con.dst.copy_span(),
        ));
        self.add_obligations(Some(cons));
    }

    /// Check that the events provided to an invoke obey the constraints implied
    /// by the component's delays.
    fn check_invoke_binds(
        &mut self,
        invoke: &core::Invoke,
        ctx: &CompBinding,
    ) -> Traverse {
        let inv_sig = ctx.get_invoke_idx(&invoke.name).resolved_signature(ctx);
        let binds = &ctx.get_invoke(&invoke.name).events;
        let this_sig = ctx.this();

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
                    "event provided to invoke triggers too often",
                    invoke.copy_span(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "provided event may trigger every {} cycles",
                        ev_delay,
                    ),
                    ev.copy_span(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "interface requires event to trigger once in {} cycles",
                        this_ev_delay,
                    ),
                    this_ev.copy_span(),
                ));
            constraints.push(cons);
        }
        self.add_obligations(constraints.into_iter());

        Traverse::Continue(())
    }

    fn check_invoke_ports(
        &mut self,
        invoke: &core::Invoke,
        ctx: &CompBinding,
    ) -> Traverse {
        // If this is a high-level invoke, check all port requirements
        if let Some(actuals) = invoke.ports.clone() {
            let inv_idx = ctx.get_invoke_idx(&invoke.name);
            // We use an unresolved signature here because [Self::connect] will eventually resolve them using
            // [CompBinding::get_resolved_ports]
            let sig = inv_idx.unresolved_signature(ctx);
            let inputs = ctx.prog[sig]
                .inputs()
                .map(|pd| pd.name.clone())
                .collect_vec();
            // Check connections implied by the invocation
            for (actual, formal) in actuals.iter().zip(inputs) {
                let dst = core::Port::comp(invoke.name.clone(), formal.clone())
                    .set_span(formal.copy_span());
                let con = core::Connect::new(dst, actual.clone(), None);
                self.connect(&con, ctx)?;
            }
        }

        Traverse::Continue(())
    }
}

impl visitor::Checker for IntervalCheck {
    fn new(ns: &core::Namespace) -> Self {
        let mut solver = FilSolver::new().unwrap();
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
                &mut diagnostics,
            );
            log::trace!("==========");
        }
        log::info!("interval-check.sigs: {}ms", t.elapsed().as_millis());

        Self::from((solver, diagnostics))
    }

    fn clear_data(&mut self) {
        self.obligations.clear();
        self.facts.clear();
        self.vars.clear();
    }

    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    fn forloop(&mut self, l: &core::ForLoop, ctx: &CompBinding) -> Traverse {
        let ForLoop {
            idx,
            start,
            end,
            body,
        } = l;

        self.add_var(idx.clone());
        let var_bounds = vec![
            core::OrderConstraint::gte(
                core::TimeSub::from(core::Expr::from(idx.clone())),
                core::TimeSub::from(start.clone()),
            )
            .into(),
            core::OrderConstraint::lt(
                core::TimeSub::from(core::Expr::from(idx.clone())),
                core::TimeSub::from(end.clone()),
            )
            .into(),
        ];
        self.add_facts(var_bounds);

        for cmd in body {
            self.command(cmd, ctx);
        }

        Traverse::Continue(())
    }

    fn connect(&mut self, con: &core::Connect, ctx: &CompBinding) -> Traverse {
        let src = &con.src;
        let dst = &con.dst;
        log::trace!("Checking connect: {} = {}", dst, src);

        // Check within-bounds access if the ports are bundles
        self.port_bundle_index(src, ctx);
        self.port_bundle_index(dst, ctx);

        let resolve_range =
            |r: &core::Range,
             event_b: &utils::Binding<Time>,
             param_b: &utils::Binding<core::Expr>| {
                r.clone().resolve_exprs(param_b).resolve_event(event_b)
            };

        let dst_port = ctx.get_resolved_port(dst, resolve_range);
        let src_port = ctx.get_resolved_port(src, resolve_range);
        self.check_width(con, &src_port, &dst_port);

        let requirement = dst_port.unwrap().liveness;
        let src_pos = src.copy_span();
        // If we have: dst = src. We need:
        // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
        // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
        if let Some(guarantee) = &src_port {
            let within_fact = OrderConstraint::subset(
                requirement.clone(),
                guarantee.liveness.clone(),
            )
            .map(|e| {
                core::Constraint::base(e)
                    .add_note(self.diag.add_info(
                        format!(
                            "source is available for {}",
                            guarantee.liveness
                        ),
                        src_pos,
                    ))
                    .add_note(self.diag.add_info(
                        format!("destination's requirement {}", requirement),
                        dst.copy_span(),
                    ))
            })
            .collect_vec();
            self.add_obligations(within_fact);
        }

        Traverse::Continue(())
    }

    fn invoke(&mut self, invoke: &core::Invoke, ctx: &CompBinding) -> Traverse {
        log::trace!("Checking: {invoke}");
        // Check the bindings for abstract variables do not violate @interface
        // requirements
        self.check_invoke_binds(invoke, ctx)?;

        // Check that the invocation's port connects are valid
        self.check_invoke_ports(invoke, ctx)?;

        // Check that the invocation's events satisfy well-formedness the component's constraints
        // XXX: We cannot replace this call with `resolved_signature` because the `well_formed` call fails.
        //      This is because the resolution process doesn't correctly change the name of the event bindings.
        let constraints: Vec<core::Constraint> = ctx
            .get_invoke_idx(&invoke.name)
            .get_resolved_sig_constraints(ctx, &mut self.diag);

        for con in constraints {
            let con_with_info = con.add_note(self.diag.add_info(
                "component's where clause constraints must be satisfied",
                invoke.copy_span(),
            ));
            self.add_obligations(iter::once(con_with_info));
        }

        Traverse::Continue(())
    }

    fn enter_component(
        &mut self,
        comp: &core::Component,
        _: &CompBinding,
    ) -> Traverse {
        // Ensure that the signature is well-formed
        let cons = comp.sig.well_formed(&mut self.diag);
        self.add_obligations(cons);

        // User-level components are not allowed to have ordering constraints.
        // See https://github.com/cucapra/filament/issues/27.
        let mut has_ulc = false;
        for constraint in &comp.sig.constraints {
            if constraint.is_time_ordering() {
                has_ulc = true;
                let err = Error::malformed(
                    "user-level component cannot have ordering constraints over events",
                )
                .add_note(self.diag.add_info(
                    format!("user-level component defines ordering between events: {constraint}"),
                    comp.sig.name.copy_span(),
                ));
                self.diag.add_error(err);
            } else {
                self.add_facts(Some(constraint.clone()))
            }
        }
        // If the component uses a user-level constraint, we cannot verify it's internal.
        if has_ulc {
            return Traverse::Break(());
        }

        Traverse::Continue(())
    }

    fn exit_component(
        &mut self,
        comp: &core::Component,
        ctx: &CompBinding,
    ) -> Traverse {
        // Add obligations from disjointness constraints
        let share = self.drain_sharing(ctx);

        // Prove all the required obligations
        let obs = self.drain_obligations();
        let t = std::time::Instant::now();
        let vars = comp
            .sig
            .events()
            .chain(comp.sig.params.iter().cloned())
            .chain(self.vars())
            .collect_vec();

        self.solver
            .prove(vars, self.facts.clone(), obs, share, &mut self.diag);
        log::info!(
            "interval-check.{}.prove: {}ms",
            comp.sig.name,
            t.elapsed().as_millis()
        );

        Traverse::Continue(())
    }
}
