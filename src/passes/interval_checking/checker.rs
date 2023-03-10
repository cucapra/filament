use super::IntervalCheck;
use crate::binding::CompBinding;
use crate::core::{self, ForLoop, OrderConstraint, Time};
use crate::errors::Error;
use crate::utils::{self, FilSolver};
use crate::visitor::{self, Checker, Traverse};
use crate::{cmdline, diagnostics};
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
            idx.inner().clone().into(),
            bun_len.inner().clone().into(),
        ))
        .obligation("bundle index must be less than bundle length")
        .add_note(
            self.diag
                .add_info(
                    format!("cannot prove within-bounds bundle access: index {idx} greater than bundle length {bun_len}"),
                    idx.pos()
                ),
        );
        let smaller = core::Constraint::sub(OrderConstraint::gte(
            idx.inner().clone().into(),
            core::Expr::from(0).into(),
        ))
        .obligation("bundle index must be greater than or equal to 0")
        .add_note(
            self.diag
                .add_info(
                    format!("cannot prove within-bounds bundle access: index {idx} less than 0"),
                    idx.pos()
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
            .map(|p| p.bitwidth.inner().clone())
            .unwrap_or_else(|| 32.into());
        let src_w = src
            .as_ref()
            .map(|p| p.bitwidth.inner().clone())
            .unwrap_or_else(|| 32.into());

        let cons = core::Constraint::sub(core::OrderConstraint::eq(
            dst_w.clone().into(),
            src_w.clone().into(),
        ))
        .obligation("source and destination widths must match")
        .add_note(self.diag.add_info(
            format!("source `{}' has width {src_w}", con.src.name()),
            con.src.pos(),
        ))
        .add_note(self.diag.add_info(
            format!("destination `{}' has width {dst_w}", con.dst.name(),),
            con.dst.pos(),
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
        for (idx, (ev_expr, pos)) in binds
            .iter()
            .zip(invoke.abstract_vars.iter().map(|t| t.pos()))
            .enumerate()
        {
            let ev = ev_expr.event();

            let this_ev = this_sig.get_event(&ev);
            let this_ev_delay = &this_ev.delay;
            let ev = &inv_sig.events[idx];
            let ev_delay = &ev.delay;

            // Generate constraint
            let cons =
                core::Constraint::sub(core::OrderConstraint::gte(
                    this_ev_delay.inner().clone(),
                    ev_delay.inner().clone(),
                ))
                .obligation("event provided to invocation triggers more often that invocation's event's delay allows")
                .add_note(self.diag.add_info(
                    "event provided to invoke triggers too often",
                    pos,
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "invocation's event is allowed to trigger every {} cycles",
                        ev_delay,
                    ),
                    ev.delay.pos(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "this event triggers every {} cycles",
                        this_ev_delay,
                    ),
                    this_ev.delay.pos(),
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
                let dst = core::Loc::new(
                    core::Port::comp(invoke.name.clone(), formal.clone()),
                    formal.pos(),
                );
                let con = core::Connect::new(dst, actual.clone(), None);
                self.connect(&con, ctx)?;
            }
        }

        Traverse::Continue(())
    }
}

impl visitor::Checker for IntervalCheck {
    fn new(opts: &cmdline::Opts, ns: &core::Namespace) -> Self {
        let mut solver = FilSolver::new(opts.show_models).unwrap();
        let mut diagnostics = diagnostics::Diagnostics::default();

        // Check that all signatures are well formed
        let t = std::time::Instant::now();
        for (_, sig) in ns.signatures() {
            log::trace!("===== Signature {} =====", &sig.name);

            let constraints = sig
                .event_constraints
                .iter()
                .map(|c| utils::SExp::from(c.inner().clone()))
                .chain(
                    sig.param_constraints
                        .iter()
                        .map(|c| utils::SExp::from(c.inner().clone())),
                )
                .collect_vec();

            solver.prove(
                sig.events()
                    .map(|e| e.take())
                    .chain(sig.params.iter().cloned()),
                constraints,
                sig.well_formed(&mut diagnostics).into_iter().collect(),
                vec![],
                &mut diagnostics,
            );
            log::trace!("==========");
        }
        log::info!("interval-check.sigs: {}ms", t.elapsed().as_millis());

        Self::new(solver, diagnostics)
    }

    fn clear_data(&mut self) {
        self.obligations.clear();
        self.facts.clear();
        self.vars.clear();
    }

    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    fn if_(&mut self, l: &core::If, ctx: &CompBinding) -> Traverse {
        let cond = utils::SExp::from(l.cond.clone());

        // Check the then branch using the condition as a path condition
        self.push_path_cond(cond.clone());
        for cmd in &l.then {
            self.command(cmd, ctx);
        }

        self.pop_path_cond();
        self.push_path_cond(!cond);
        for cmd in &l.alt {
            self.command(cmd, ctx);
        }

        Traverse::Continue(())
    }

    fn forloop(&mut self, l: &core::ForLoop, ctx: &CompBinding) -> Traverse {
        let ForLoop {
            idx,
            start,
            end,
            body,
        } = l;

        self.add_var(*idx);
        let var_bounds = vec![
            core::OrderConstraint::gte(
                core::TimeSub::from(core::Expr::from(*idx)),
                core::TimeSub::from(start.clone()),
            )
            .into(),
            core::OrderConstraint::lt(
                core::TimeSub::from(core::Expr::from(*idx)),
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

    fn instance(
        &mut self,
        inst: &core::Instance,
        ctx: &CompBinding,
    ) -> Traverse {
        // Check that the binding parameters provided to the instance are well-formed
        let inst_idx = ctx.get_instance_idx(&inst.name);
        let sig = inst_idx.param_resolved_signature(ctx);
        let cons = sig
            .param_constraints
            .into_iter()
            .map(|c| {
                let pos = c.pos();
                c.take()
                    .obligation("instantiation violates component's constraint")
                    .add_note(
                        self.diag.add_info("constraint was violated", pos),
                    )
                    .add_note(
                        self.diag.add_info(
                            "instantiation occurs here",
                            inst.name.pos(),
                        ),
                    )
            })
            .collect_vec();
        self.add_obligations(cons);
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
        // If we have: dst = src. We need:
        // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
        // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
        if let Some(guarantee) = &src_port {
            let within_fact = OrderConstraint::subset(
                requirement.clone().take(),
                guarantee.liveness.clone().take(),
            )
            .map(|e| {
                core::Constraint::base(e)
                    .obligation("source port must be available longer than the destination port requires")
                    .add_note(self.diag.add_info(
                        format!(
                            "source is available for {}",
                            guarantee.liveness
                        ),
                        src.pos(),
                    ))
                    .add_note(self.diag.add_info(
                        format!("destination's requirement {}", requirement),
                        dst.pos(),
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
        let constraints = ctx
            .get_invoke_idx(&invoke.name)
            .resolved_event_constraints(ctx);

        for con in constraints {
            let pos = con.pos();
            // XXX(rachit): Attach location information for constraints
            let con_with_info = con
                .take()
                .obligation("invocation violates component's constraint")
                .add_note(self.diag.add_info("constraint was violated", pos))
                .add_note(self.diag.add_info(
                    "invocation violates component's constraint",
                    invoke.name.pos(),
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
        log::trace!("=========== Component {} ==========", comp.sig.name);
        // Ensure that the signature is well-formed
        let cons = comp.sig.well_formed(&mut self.diag);
        self.add_obligations(cons);

        // User-level components are not allowed to have ordering constraints over events.
        // See https://github.com/cucapra/filament/issues/27.
        let mut has_ulc = false;
        for constraint in &comp.sig.event_constraints {
            if !constraint.is_eq() {
                has_ulc = true;
                let err = Error::malformed(
                    "user-level component cannot have ordering constraints over events",
                )
                .add_note(self.diag.add_info(
                    format!("user-level component defines ordering between events: {constraint}"),
                    constraint.pos(),
                ));
                self.diag.add_error(err);
            } else {
                self.add_facts(vec![utils::SExp::from(
                    constraint.inner().clone(),
                )])
            }
        }

        // Add all parameter constraints
        self.add_facts(
            comp.sig
                .param_constraints
                .iter()
                .map(|c| utils::SExp::from(c.inner().clone())),
        );

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
            .map(|e| e.take())
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
