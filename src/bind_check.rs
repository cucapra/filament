use crate::{
    core::{self, TimeRep, WidthRep},
    errors::{Error, FilamentResult, WithPos},
    visitor,
};
use itertools::Itertools;

pub struct BindCheck;

impl<T: TimeRep, W: WidthRep> visitor::Checker<T, W> for BindCheck {
    fn new(_: &core::Namespace<T, W>) -> FilamentResult<Self> {
        Ok(Self)
    }

    fn clear_data(&mut self) {}

    fn enter_component(
        &mut self,
        comp: &core::Component<T, W>,
        _ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Self::signature(&comp.sig)
    }

    fn external(
        &mut self,
        sig: &core::Signature<T, core::PortParam>,
    ) -> FilamentResult<()> {
        Self::signature(sig)
    }

    fn instance(
        &mut self,
        inst: &core::Instance<W>,
        ctx: &visitor::CompBinding<T, W>,
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
            return Err(
                Error::malformed(msg.clone()).add_note(msg, inst.copy_span())
            );
        }

        Ok(())
    }

    fn invoke(
        &mut self,
        inv: &core::Invoke<T>,
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Self::bind_invoke(inv, ctx)?;
        let sig = ctx.get_invoke_sig(&inv.name);
        let events = ctx.prog.events(sig);
        if let Some(ports) = &inv.ports {
            // Check that scheduling events are bound
            for time in &inv.abstract_vars {
                let ev = time.event();
                if !events.iter().any(|e| e.event == ev) {
                    return Err(Error::undefined(ev.clone(), "Event")
                        .add_note("Event is not bound", ev.copy_span()));
                }
            }

            // Check that the number of ports matches the number of ports
            // XXX(rachit): We can directly count the number of inputs by defining a method on
            // signatures
            let inputs = ctx.prog.input_names(sig);
            let formals = inputs.len();
            let actuals = ports.len();
            if formals != actuals {
                return Err(Error::malformed(format!(
                    "Invoke of {} requires {formals} ports but {actuals} were provided",
                    inv.instance,
                ))
                .add_note("Instance used here", inv.copy_span()));
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
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<()> {
        let resolve =
            |r: &core::Range<T>, _: &core::Binding<T>, _: &core::Binding<W>| {
                r.clone()
            };
        let dst_w = ctx
            .get_resolved_port(&con.dst, resolve)
            .map(|p| p.bitwidth)
            .unwrap_or_else(|| W::concrete(32));
        let src_w = ctx
            .get_resolved_port(&con.src, resolve)
            .map(|p| p.bitwidth)
            .unwrap_or_else(|| W::concrete(32));

        if dst_w != src_w {
            return Err(Error::malformed("Port width mismatch".to_string())
                .add_note(
                    format!("Source `{}' has width {src_w}", con.src.name()),
                    con.src.copy_span(),
                )
                .add_note(
                    format!(
                        "Destination `{}' has width {dst_w}",
                        con.dst.name(),
                    ),
                    con.dst.copy_span(),
                ));
        }
        Ok(())
    }
}

impl BindCheck {
    /// Check that an invoke's instance is bound and and bind its signature
    fn bind_invoke<T: TimeRep, W: WidthRep>(
        inv: &core::Invoke<T>,
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<()> {
        let sig = ctx.get_invoke_sig(&inv.name);
        // Check that the number of arguments is more than the minimum number of required formals
        let min_formals = ctx
            .prog
            .events(sig)
            .iter()
            .take_while(|eb| eb.default.is_none())
            .count();
        let max_formals = ctx.prog.events(sig).len();
        let actuals = inv.abstract_vars.len();
        if min_formals > actuals {
            return Err(Error::malformed(format!(
                "Invoke of {} requires at least {min_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(
                format!("Invoke requires at least {min_formals} events but {actuals} are provided"),
                inv.instance.copy_span()
            ));
        } else if actuals > max_formals {
            return Err(Error::malformed(format!(
                "Invoke of {} requires at most {max_formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(
                format!("Invoke accepts at most {max_formals} events but {actuals} are provided"),
                inv.instance.copy_span()
            ));
        }

        Ok(())
    }

    /// Check the binding of a component
    fn signature<T: TimeRep, W: WidthRep>(
        sig: &core::Signature<T, W>,
    ) -> FilamentResult<()> {
        let events = sig.events().collect_vec();
        // Check all the definitions only use bound events
        for pd in sig.ports() {
            for time in pd.liveness.events() {
                let ev = time.event();
                if !events.contains(&ev) {
                    return Err(Error::undefined(ev.clone(), "event")
                        .add_note(
                            "Event is not defined in the signature",
                            ev.copy_span(),
                        ));
                }
            }
        }
        // Check that interface ports use only bound events
        for id in &sig.interface_signals {
            if !events.contains(&id.event) {
                return Err(Error::undefined(id.event.clone(), "event")
                    .add_note(
                        "Event is not defined in the signature",
                        id.event.copy_span(),
                    ));
            }
        }
        // Check constraints use bound events
        for constraint in &sig.constraints {
            for ev in constraint.events() {
                if !events.contains(&ev) {
                    return Err(Error::undefined(ev.clone(), "event")
                        .add_note(
                            "Event is not defined in the signature",
                            ev.copy_span(),
                        ));
                }
            }
        }
        Ok(())
    }
}
