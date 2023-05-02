use std::collections::HashSet;

use crate::{
    binding::{self, InvIdx},
    cmdline,
    core::{self, Loc},
    diagnostics,
    errors::Error,
    utils::GPosIdx,
    visitor::{self, Traverse},
};
use itertools::Itertools;

pub struct BindCheck {
    /// Currently bound parameters. All parameters are considered visible in a
    /// component and we don't allow shadowing.
    vars: Vec<Loc<core::Id>>,
    /// Currently bound events
    events: Vec<core::Id>,
    /// Current set of diagnostics
    diag: diagnostics::Diagnostics,
}

impl BindCheck {
    /// Push a new set of bound variables and return the number of variables added
    fn add_vars(&mut self, vars: &[Loc<core::Id>]) {
        self.vars.extend_from_slice(vars);
    }

    /// Check if the given variable is bound
    fn has_var(&self, var: &core::Id) -> bool {
        self.vars.iter().any(|v| *v.inner() == *var)
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
            if !self.has_var(abs) {
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
        _is_port: bool,
        bun: &core::Bundle,
        _bind: &binding::CompBinding,
    ) -> Traverse {
        let core::BundleType {
            idx,
            len,
            liveness,
            bitwidth,
        } = &bun.typ;
        self.add_vars(&[idx.clone()]);
        for time in liveness.time_exprs() {
            self.time(time, liveness.pos());
        }
        for expr in &[bitwidth, len] {
            self.expr(expr, expr.pos());
        }
        Traverse::Continue(())
    }

    /// Check the binding of a component
    fn signature(&mut self, sig: &core::Signature) -> Traverse {
        let events = sig.events().collect_vec();
        let params = &sig.params;
        self.add_vars(params);
        self.events.extend(events.iter().map(|ev| *ev.inner()));
        // Check all the definitions only use bound events and parameters
        for pd in sig.ports() {
            if let core::PortDef::Port { liveness, .. } = pd.inner() {
                for time in liveness.time_exprs() {
                    self.time(time, liveness.pos());
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
        self.add_vars(&[l.idx.clone()]);
        for cmd in &l.body {
            self.command(cmd, ctx);
        }
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

    fn exit_component(
        &mut self,
        _: &core::Component,
        _ctx: &binding::CompBinding,
    ) -> Traverse {
        // Find all duplicate bindings for parameters and report them
        let mut defined = HashSet::with_capacity(self.vars.len());
        for v in &self.vars {
            if !defined.insert(v) {
                let old = defined.get(v).unwrap();
                let err = Error::malformed(format!(
                    "duplicate binding for parameter `{}`",
                    v
                ))
                .add_note(self.diag.add_info("duplicate binding here", v.pos()))
                .add_note(self.diag.add_info("first binding here", old.pos()));
                self.diag.add_error(err);
            }
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
