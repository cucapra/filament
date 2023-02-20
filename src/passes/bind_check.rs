use crate::{
    binding::{self, InvIdx},
    core, diagnostics,
    errors::{Error, WithPos},
    utils,
    visitor::{self, Traverse},
};
use itertools::Itertools;

pub struct BindCheck {
    // Currently bound parameters
    vars: Vec<core::Id>,
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
    fn time(
        &mut self,
        time: &core::Time,
        events: &[core::Id],
        params: &[core::Id],
    ) {
        // Check that events are bound
        let ev = &time.event;
        if !events.contains(ev) {
            let err = Error::undefined(ev.clone(), "event").add_note(
                self.diag.add_info(
                    "event is not defined in the signature",
                    ev.copy_span(),
                ),
            );
            self.diag.add_error(err);
        }

        // Check that the abstract variables are bound
        self.expr(time.offset(), params);
    }

    fn expr(&mut self, expr: &core::Expr, params: &[core::Id]) {
        for abs in expr.exprs() {
            if !params.iter().chain(self.vars.iter()).contains(abs) {
                let err = Error::undefined(abs.clone(), "parameter").add_note(
                    self.diag.add_info(
                        "parameter is not defined in the signature",
                        abs.copy_span(),
                    ),
                );
                self.diag.add_error(err);
            }
        }
    }
}

impl visitor::Checker for BindCheck {
    fn new(_: &core::Namespace) -> Self {
        Self {
            diag: diagnostics::Diagnostics::default(),
            vars: Vec::new(),
        }
    }

    fn clear_data(&mut self) {
        self.vars.clear();
    }

    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    /// Check the binding of a component
    fn signature(&mut self, sig: &core::Signature) -> Traverse {
        let events = sig.events().collect_vec();
        let params = &sig.params;
        // Check all the definitions only use bound events and parameters
        for pd in sig.ports() {
            for time in pd.liveness.time_exprs() {
                self.time(time, &events, params);
            }
            self.expr(&pd.bitwidth, params);
        }
        // Check that interface ports use only bound events
        for id in &sig.interface_signals {
            if !events.contains(&id.event) {
                let err = Error::undefined(id.event.clone(), "event").add_note(
                    self.diag.add_info(
                        "event is not defined in the signature",
                        id.event.copy_span(),
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
                self.time(time, &events, params)
            }
            for expr in delay.exprs() {
                self.expr(expr, params);
            }
        }

        // Check constraints use bound events and parameters
        for constraint in &sig.constraints {
            // XXX: Same problem as the loop above
            for time in constraint.events() {
                self.time(time, &events, params)
            }
            for expr in constraint.exprs() {
                self.expr(expr, params);
            }
        }
        Traverse::Continue(())
    }

    fn forloop(
        &mut self,
        l: &core::ForLoop,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        let vars = self.push_vars(&[l.idx.clone()]);
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
            self.expr(param, &ctx.this().params);
        }

        if param_len != inst.bindings.len() {
            let msg = format!(
                "`{}' requires {} parameters but {} were provided",
                inst.component,
                param_len,
                inst.bindings.len(),
            );
            let err = Error::malformed(msg.clone())
                .add_note(self.diag.add_info(msg, inst.copy_span()));
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

        let this_sig = ctx.this();
        let this_events = this_sig.events().collect_vec();
        let this_params = &this_sig.params;
        if let Some(ports) = &inv.ports {
            // Check that scheduling events are bound
            for time in &inv.abstract_vars {
                self.time(time, &this_events, this_params);
            }

            // Check that the number of ports matches the number of ports
            let inputs = ctx.prog[sig]
                .inputs()
                .map(|pd| pd.name.clone())
                .collect_vec();
            let formals = inputs.len();
            let actuals = ports.len();
            if formals != actuals {
                let err = Error::malformed(format!(
                    "Invoke of {} requires {formals} ports but {actuals} were provided",
                    inv.instance,
                )).add_note(self.diag.add_info("instance used here", inv.copy_span()));
                self.diag.add_error(err);
            }

            // Check the connections implied by the ports
            for (actual, formal) in ports.iter().zip(inputs) {
                let dst = core::Port::comp(inv.name.clone(), formal.clone())
                    .set_span(formal.copy_span());
                let con = core::Connect::new(dst, actual.clone(), None);
                self.connect(&con, ctx)?;
            }
        }
        Traverse::Continue(())
    }

    fn connect(
        &mut self,
        con: &core::Connect,
        ctx: &binding::CompBinding,
    ) -> Traverse {
        let resolve =
            |r: &core::Range,
             _: &utils::Binding<core::Time>,
             _: &utils::Binding<core::Expr>| r.clone();
        let dst_w = ctx
            .get_resolved_port(&con.dst, resolve)
            .map(|p| p.bitwidth)
            .unwrap_or_else(|| 32.into());
        let src_w = ctx
            .get_resolved_port(&con.src, resolve)
            .map(|p| p.bitwidth)
            .unwrap_or_else(|| 32.into());

        if dst_w != src_w {
            let err = Error::malformed("port width mismatch".to_string())
                .add_note(self.diag.add_info(
                    format!("source `{}' has width {src_w}", con.src.name()),
                    con.src.copy_span(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "destination `{}' has width {dst_w}",
                        con.dst.name(),
                    ),
                    con.dst.copy_span(),
                ));
            self.diag.add_error(err);
        }
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
                inv.instance.copy_span()
            ));
            self.diag.add_error(err);
        } else if actuals > max_formals {
            let err = Error::malformed(format!(
                "invoke of {} requires at most {max_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(self.diag.add_info(
                format!("invoke accepts at most {max_formals} events but {actuals} are provided"),
                inv.instance.copy_span()
            ));
            self.diag.add_error(err);
        }

        inv_idx
    }
}
