use crate::{
    core, diagnostics,
    errors::{Error, FilamentResult, WithPos},
    utils, visitor,
};
use itertools::Itertools;

pub struct BindCheck {
    diag: diagnostics::Diagnostics,
}

impl visitor::Checker for BindCheck {
    fn new(_: &core::Namespace) -> FilamentResult<Self> {
        Ok(Self {
            diag: diagnostics::Diagnostics::default(),
        })
    }

    fn clear_data(&mut self) {}

    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    /// Check the binding of a component
    fn signature(&mut self, sig: &core::Signature) -> FilamentResult<()> {
        let events = sig.events().collect_vec();
        // Check all the definitions only use bound events
        for pd in sig.ports() {
            for time in pd.liveness.time_exprs() {
                let ev = &time.event;
                if !events.contains(ev) {
                    return Err(Error::undefined(ev.clone(), "event")
                        .add_note(self.diag.add_info(
                            "Event is not defined in the signature",
                            ev.copy_span(),
                        )));
                }
            }
        }
        // Check that interface ports use only bound events
        for id in &sig.interface_signals {
            if !events.contains(&id.event) {
                return Err(Error::undefined(id.event.clone(), "event")
                    .add_note(self.diag.add_info(
                        "Event is not defined in the signature",
                        id.event.copy_span(),
                    )));
            }
        }
        // Check constraints use bound events
        for constraint in &sig.constraints {
            for ev in constraint.events() {
                if !events.contains(&ev) {
                    return Err(Error::undefined(ev.clone(), "event")
                        .add_note(self.diag.add_info(
                            "Event is not defined in the signature",
                            ev.copy_span(),
                        )));
                }
            }
        }
        Ok(())
    }

    fn instance(
        &mut self,
        inst: &core::Instance,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        let bound = ctx.get_instance(&inst.name);
        let param_len = ctx.prog.map_signature(
            bound.sig,
            |e| e.params.len(),
            |c| c.params.len(),
        );

        if param_len != inst.bindings.len() {
            let msg = format!(
                "`{}' requires {} bindings but {} were provided",
                inst.component,
                param_len,
                inst.bindings.len(),
            );
            let err = Error::malformed(msg.clone())
                .add_note(self.diag.add_info(msg, inst.copy_span()));
            self.diag.add_error(err);
        }

        Ok(())
    }

    fn invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<()> {
        let inv_idx = self.bind_invoke(inv, ctx)?;
        let sig = inv_idx.unresolved_signature(ctx);

        let this_sig = ctx.this();
        let this_events = &this_sig.events;
        if let Some(ports) = &inv.ports {
            // Check that scheduling events are bound
            for time in &inv.abstract_vars {
                let ev = time.event();
                if !this_events.iter().any(|e| e.event == ev) {
                    let err = Error::undefined(ev.clone(), "Event").add_note(
                        self.diag
                            .add_info("Event is not bound", ev.copy_span()),
                    );
                    self.diag.add_error(err);
                }
            }

            // Check that the number of ports matches the number of ports
            // XXX(rachit): We can directly count the number of inputs by defining a method on
            // signatures
            let inputs = ctx.prog.input_names(sig);
            let formals = inputs.len();
            let actuals = ports.len();
            if formals != actuals {
                let err = Error::malformed(format!(
                    "Invoke of {} requires {formals} ports but {actuals} were provided",
                    inv.instance,
                )).add_note(self.diag.add_info("Instance used here", inv.copy_span()));
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
        Ok(())
    }

    fn connect(
        &mut self,
        con: &core::Connect,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<()> {
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
            let err = Error::malformed("Port width mismatch".to_string())
                .add_note(self.diag.add_info(
                    format!("Source `{}' has width {src_w}", con.src.name()),
                    con.src.copy_span(),
                ))
                .add_note(self.diag.add_info(
                    format!(
                        "Destination `{}' has width {dst_w}",
                        con.dst.name(),
                    ),
                    con.dst.copy_span(),
                ));
            self.diag.add_error(err);
        }
        Ok(())
    }
}

impl BindCheck {
    /// Check that an invoke's instance is bound and and bind its signature
    fn bind_invoke(
        &mut self,
        inv: &core::Invoke,
        ctx: &visitor::CompBinding,
    ) -> FilamentResult<visitor::InvIdx> {
        let inv_idx = ctx
            .get_invoke_idx(&inv.name)
            .unwrap_or_else(|| unreachable!("Instance is not bound. BindingCtx construction should have failed."));

        let sig = inv_idx.unresolved_signature(ctx);
        // Check that the number of arguments is more than the minimum number of required formals
        let sig = ctx.prog.sig(sig);
        let min_formals = sig
            .events
            .iter()
            .take_while(|eb| eb.default.is_none())
            .count();
        let max_formals = sig.events.len();
        let actuals = inv.abstract_vars.len();
        if min_formals > actuals {
            let err = Error::malformed(format!(
                "Invoke of {} requires at least {min_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(self.diag.add_info(
                format!("Invoke requires at least {min_formals} events but {actuals} are provided"),
                inv.instance.copy_span()
            ));
            self.diag.add_error(err);
        } else if actuals > max_formals {
            let err = Error::malformed(format!(
                "Invoke of {} requires at most {max_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(self.diag.add_info(
                format!("Invoke accepts at most {max_formals} events but {actuals} are provided"),
                inv.instance.copy_span()
            ));
            self.diag.add_error(err);
        }

        Ok(inv_idx)
    }
}
