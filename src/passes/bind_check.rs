use crate::{
    binding::{self, InvIdx},
    cmdline, core, diagnostics,
    errors::Error,
    utils::{self, GPosIdx},
    visitor::{self, Traverse},
};
use itertools::Itertools;

pub struct BindCheck {
    // Currently bound parameters
    vars: Vec<core::Id>,
    // Currently bound events
    events: Vec<core::Id>,
    diag: diagnostics::Diagnostics,
}

impl BindCheck {
    /// Push a new set of bound variables and return the number of variables added
    fn push_vars(&mut self, vars: &[core::Id]) -> usize {
        let n = vars.len();
        self.vars.extend_from_slice(vars);
        n
    }

    /// Remove the last `n` variables
    fn pop_vars(&mut self, n: usize) {
        self.vars.truncate(self.vars.len() - n);
    }

    /// Check that a time expression is well-formed
    fn time(&mut self, time: &core::Time, pos: GPosIdx) {
        // Check that events are bound
        let ev = &time.event;
        if !self.events.contains(ev) {
            let err =
                Error::undefined(*ev, "event").add_note(self.diag.add_info(
                    format!("event `{ev}' is not defined in the signature"),
                    pos,
                ));
            self.diag.add_error(err);
        }

        // Check that the abstract variables are bound
        self.expr(time.offset(), pos);
    }

    fn expr(&mut self, expr: &core::Expr, pos: GPosIdx) {
        for abs in expr.exprs() {
            if !self.vars.iter().chain(self.vars.iter()).contains(abs) {
                let err = Error::undefined(*abs, "parameter").add_note(
                    self.diag.add_info(
                        format!(
                            "parameter `{abs}' is not defined in the signature"
                        ),
                        pos,
                    ),
                );
                self.diag.add_error(err);
            }
        }
    }
}

impl visitor::Checker for BindCheck {
    fn new(_: &cmdline::Opts, _: &core::Namespace) -> Self {
        Self {
            diag: diagnostics::Diagnostics::default(),
            vars: Vec::new(),
            events: Vec::new(),
        }
    }

    fn clear_data(&mut self) {
        self.vars.clear();
        self.events.clear();
    }

    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    fn bundle(
        &mut self,
        bun: &core::Bundle,
        _: &binding::CompBinding,
    ) -> Traverse {
        let core::BundleType {
            idx,
            len,
            liveness,
            bitwidth,
        } = &bun.typ;
        let n = self.push_vars(&[*idx.inner()]);
        for time in liveness.time_exprs() {
            self.time(time, liveness.pos());
        }
        for expr in &[bitwidth, len] {
            self.expr(expr, expr.pos());
        }
        self.pop_vars(n);
        Traverse::Continue(())
    }

    /// Check the binding of a component
    fn signature(&mut self, sig: &core::Signature) -> Traverse {
        let events = sig.events().collect_vec();
        let params = &sig.params;
        self.push_vars(params);
        self.events.extend(events.iter().map(|ev| *ev.inner()));
        // Check all the definitions only use bound events and parameters
        for pd in sig.ports() {
            match pd.inner() {
                core::PortDef::Port { liveness, .. } => {
                    for time in liveness.time_exprs() {
                        self.time(time, liveness.pos());
                    }
                }
                core::PortDef::Bundle(core::Bundle {
                    typ: core::BundleType { idx, liveness, .. },
                    ..
                }) => {
                    let n = self.push_vars(&[*idx.inner()]);
                    for time in liveness.time_exprs() {
                        self.time(time, liveness.pos());
                    }
                    self.pop_vars(n);
                }
            }
            self.expr(pd.bitwidth(), pd.bitwidth().pos());
        }
        // Check that interface ports use only bound events
        for id in &sig.interface_signals {
            if !self.events.contains(&id.event) {
                let err = Error::undefined(id.event, "event").add_note(
                    self.diag.add_message(
                        "event is not defined in the signature",
                        // id.event.copy_span(),
                    ),
                );
                self.diag.add_error(err);
            }
        }

        // Check that the event delays use bound parameters
        for eb in &sig.events {
            let delay = &eb.delay;
            // XXX: This check does duplicated work because if a TimeSub has
            // time in it, the first loop will get all the expressions as well.
            for time in delay.events() {
                self.time(time, delay.pos())
            }
            for expr in delay.exprs() {
                self.expr(expr, delay.pos());
            }
        }

        // Check the parameter constraints
        for constraint in &sig.param_constraints {
            let cons = constraint.inner();
            self.expr(&cons.left, constraint.pos());
            self.expr(&cons.right, constraint.pos());
        }

        // Check constraints use bound events and parameters
        for constraint in &sig.event_constraints {
            // XXX: Same problem as the loop above
            let cons = constraint.inner();
            self.time(&cons.left, constraint.pos());
            self.time(&cons.right, constraint.pos());

            self.expr(cons.left.offset(), constraint.pos());
            self.expr(cons.right.offset(), constraint.pos());
        }

        Traverse::Continue(())
    }

    fn forloop(
        &mut self,
        l: &core::ForLoop,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        let vars = self.push_vars(&[l.idx]);
        for cmd in &l.body {
            self.command(cmd, ctx);
        }
        self.pop_vars(vars);
        Traverse::Continue(())
    }

    fn instance(
        &mut self,
        inst: &core::Instance,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        let bound = ctx.get_instance(&inst.name);
        let param_len = ctx.prog[bound.sig].params.len();
        for param in &inst.bindings {
            self.expr(param, param.pos());
        }

        if param_len != inst.bindings.len() {
            let msg = format!(
                "`{}' requires {} parameters but {} were provided",
                inst.component,
                param_len,
                inst.bindings.len(),
            );
            let err = Error::malformed(msg.clone())
                .add_note(self.diag.add_info(msg, inst.component.pos()));
            self.diag.add_error(err);
        }

        Traverse::Continue(())
    }

    fn invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        let inv_idx = self.bind_invoke(inv, ctx);
        let sig = inv_idx.unresolved_signature(ctx);

        if let Some(ports) = &inv.ports {
            // Check that scheduling events are bound
            for time in &inv.abstract_vars {
                self.time(time, time.pos());
            }

            // Check that the number of ports matches the number of ports
            let inputs = ctx.prog[sig]
                .inputs()
                .map(|pd| pd.name().clone())
                .collect_vec();
            let formals = inputs.len();
            let actuals = ports.len();
            if formals != actuals {
                let err = Error::malformed(format!(
                    "Invoke of {} requires {formals} ports but {actuals} were provided",
                    inv.instance,
                )).add_note(self.diag.add_info("instance used here", inv.instance.pos()));
                self.diag.add_error(err);
            }

            // Check the connections implied by the ports
            for (actual, formal) in ports.iter().zip(inputs) {
                let dst = core::Loc::new(
                    core::Port::comp(inv.name.clone(), formal.clone()),
                    formal.pos(),
                );
                let con = core::Connect::new(dst, actual.clone(), None);
                self.connect(&con, ctx)?;
            }
        }
        Traverse::Continue(())
    }

    fn connect(
        &mut self,
        _con: &core::Connect,
        _ctx: &binding::CompBinding,
    ) -> Traverse {
        let _resolve =
            |r: &core::Range,
             _: &utils::Binding<core::Time>,
             _: &utils::Binding<core::Expr>| r.clone();
        // let dst_w = ctx
        //     .get_resolved_port(&con.dst, resolve)
        //     .map(|p| p.bitwidth)
        //     .unwrap_or_else(|| 32.into());
        // let src_w = ctx
        //     .get_resolved_port(&con.src, resolve)
        //     .map(|p| p.bitwidth)
        //     .unwrap_or_else(|| 32.into());

        // XXX(rachit): This cannot be checked locally. We need to generate constraints in the interval checker to check this property.
        // if dst_w != ss {
        //     let err = Error::malformed("port width mismatch".to_string())
        //         .add_note(self.diag.add_info(
        //             format!("source `{}' has width {ss}", con.src.name()),
        //             con.src.copy_span(),
        //         ))
        //         .add_note(self.diag.add_info(
        //             format!("destination `{}' has width {ds}", con.dst.name(),),
        //             con.dst.copy_span(),
        //         ));
        //     self.diag.add_error(err);
        // }
        Traverse::Continue(())
    }
}

impl BindCheck {
    /// Check that an invoke's instance is bound and and bind its signature
    fn bind_invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &binding::CompBinding,
    ) -> InvIdx {
        let inv_idx = ctx.get_invoke_idx(&inv.name);
        let sig = inv_idx.unresolved_signature(ctx);
        // Check that the number of arguments is more than the minimum number of required formals
        let sig = &ctx.prog[sig];
        let min_formals = sig
            .events
            .iter()
            .take_while(|eb| eb.default.is_none())
            .count();
        let max_formals = sig.events.len();
        let actuals = inv.abstract_vars.len();
        if min_formals > actuals {
            let err = Error::malformed(format!(
                "invoke of {} requires at least {min_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(self.diag.add_info(
                format!("invoke requires at least {min_formals} events but {actuals} are provided"),
                inv.instance.pos()
            ));
            self.diag.add_error(err);
        } else if actuals > max_formals {
            let err = Error::malformed(format!(
                "invoke of {} requires at most {max_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(self.diag.add_info(
                format!("invoke accepts at most {max_formals} events but {actuals} are provided"),
                inv.instance.pos()
            ));
            self.diag.add_error(err);
        }

        inv_idx
    }
}
