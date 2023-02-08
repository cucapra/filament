use crate::{
    ast::param as ast,
    core,
    errors::{Error, FilamentResult, WithPos},
    utils::BindMap,
    visitor::{Bindings, ResolvedInstance},
};
use itertools::Itertools;
use std::collections::HashMap;

const THIS: &str = "_this";

struct BindCheck<'a> {
    /// Bound events
    events: Vec<ast::Id>,
    /// Bound instances
    instances: BindMap<ResolvedInstance<'a>>,
    // mapping from name of invocations to the instance that they invoke
    invocations: BindMap<ast::Id>,
}

/// Transform the givun AST
impl BindCheck<'_> {
    fn new(events: Vec<ast::Id>) -> Self {
        Self {
            events,
            instances: BindMap::new(),
            invocations: BindMap::new(),
        }
    }

    // Checks if the port is well defined and returns its width.
    fn port<const INPUT: bool>(
        &mut self,
        port: &ast::Port,
    ) -> FilamentResult<ast::PortParam> {
        let check_port = |instance: &ast::Id,
                          p: &ast::Id|
         -> FilamentResult<ast::PortParam> {
            let sig = self
                .instances
                .find(instance)
                .expect("THIS component is not defined")
                .resolve()?;
            let mut iter: Box<dyn Iterator<Item = _>> = if INPUT {
                Box::new(sig.inputs())
            } else {
                Box::new(sig.outputs())
            };
            let kind = if INPUT { "input" } else { "output" };
            iter.find(|p1| p1.name == p)
                .map(|p| p.bitwidth.clone())
                // XXX(rachit): Always search interface ports regardless of input or output because we don't
                // correctly reverse them.
                .or_else(|| {
                    sig.interface_signals
                        .iter()
                        .find(|def| def.name == p)
                        .map(|_| ast::PortParam::Const(1))
                })
                .ok_or_else(|| {
                    Error::undefined(ast::Id::from(format!("{port}")), kind)
                        .add_note("Port is not defined", port.copy_span())
                })
        };

        match &port.typ {
            ast::PortType::ThisPort(p) => check_port(&ast::Id::from(THIS), p),
            ast::PortType::InvPort { invoke, name } => {
                let inst = self.invocations.find(invoke).map_err(|err| {
                    err.add_note(
                        "Invocation is not defined",
                        invoke.copy_span(),
                    )
                })?;
                check_port(inst, name)
            }
            ast::PortType::Constant(_) => Ok(ast::PortParam::Const(32)),
        }
    }

    fn connect(
        &mut self,
        dst: &ast::Port,
        src: &ast::Port,
    ) -> FilamentResult<()> {
        let dst_w = self.port::<true>(dst)?;
        let src_w = self.port::<false>(src)?;
        if dst_w != src_w {
            return Err(Error::malformed("Port width mismatch".to_string())
                .add_note(
                    format!("Source `{}' has width {src_w}", src.name()),
                    src.copy_span(),
                )
                .add_note(
                    format!("Destination `{}' has width {dst_w}", dst.name(),),
                    dst.copy_span(),
                ));
        }
        Ok(())
    }

    /// Check that an invoke's instance is bound and and bind its signature
    fn bind_invoke(&mut self, inv: &ast::Invoke) -> FilamentResult<()> {
        self.invocations
            .add(inv.bind.clone(), inv.instance.clone())?;
        // Get the signature for the instance
        let inst = self.instances.find(&inv.instance).map_err(|_| {
            Error::undefined(inv.instance.clone(), "Instance")
                .add_note("Undefined instance", inv.instance.copy_span())
        })?;

        // Check that the number of arguments is more than the minimum number of required formals
        let min_formals = inst
            .abstract_vars()
            .iter()
            .take_while(|eb| eb.default.is_none())
            .count();
        let max_formals = inst.abstract_vars().len();
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

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    fn check_invoke(&mut self, inv: &ast::Invoke) -> FilamentResult<()> {
        let inst = self.instances.get(&inv.instance);
        if let Some(ports) = &inv.ports {
            // Check that scheduling events are bound
            for time in &inv.abstract_vars {
                let ev = &time.event;
                if !self.events.contains(ev) {
                    return Err(Error::undefined(ev.clone(), "Event")
                        .add_note("Event is not bound", ev.copy_span()));
                }
            }

            // Check that the number of ports matches the number of ports
            // XXX(rachit): We can directly count the number of inputs by defining a method on
            // signatures
            let formals = inst.input_names().len();
            let actuals = ports.len();
            if formals != actuals {
                return Err(Error::malformed(format!(
                    "Invoke of {} requires {formals} ports but {actuals} were provided",
                    inv.instance,
                ))
                .add_note("Instance used here", inv.copy_span()));
            }

            // Check the connections implied by the ports
            let sig = inst.resolve()?;
            for (actual, formal) in ports.iter().zip(sig.inputs()) {
                let dst =
                    ast::Port::comp(inv.bind.clone(), formal.name.clone())
                        .set_span(formal.copy_span());
                self.connect(&dst, actual)?;
            }
        }

        Ok(())
    }

    /// Check the binding of a component
    fn check_sig(sig: &ast::Signature) -> FilamentResult<()> {
        let events = sig.events().collect_vec();
        // Check all the definitions only use bound events
        for pd in sig.ports() {
            for time in pd.liveness.events() {
                let ev = &time.event;
                if !events.contains(ev) {
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

    /// Perform the component traversal
    fn component(
        comp: &ast::Component,
        binds: &Bindings,
    ) -> FilamentResult<()> {
        // Check this signature
        Self::check_sig(&comp.sig)?;

        // Binding for instances
        let mut bind_check = BindCheck::new(
            comp.sig.events.iter().map(|e| e.event.clone()).collect(),
        );

        // Add THIS instance
        let this_sig = comp.sig.reversed();
        bind_check
            .instances
            .add(ast::Id::from(THIS), ResolvedInstance::this(&this_sig))?;

        // Create all invoke bindings
        for cmd in &comp.body {
            match cmd {
                core::Command::Invoke(inv) => bind_check.bind_invoke(inv)?,
                core::Command::Instance(inst) => {
                    let sig = binds
                        .find_component(&inst.component, &inst.bindings)
                        .map_err(|err| {
                            err.add_note("For this instance", inst.copy_span())
                        })?;
                    if sig.sig().params.len() != inst.bindings.len() {
                        let msg = format!(
                            "`{}' requires {} bindings but {} were provided",
                            inst.component,
                            sig.sig().params.len(),
                            inst.bindings.len(),
                        );
                        return Err(Error::malformed(msg.clone())
                            .add_note(msg, inst.copy_span()));
                    }
                    bind_check.instances.add(inst.name.clone(), sig)?;
                }
                core::Command::Connect(_) | core::Command::Fsm(_) => (),
            }
        }

        // Check connections and invocation port uses
        for cmd in &comp.body {
            match cmd {
                core::Command::Invoke(inv) => bind_check.check_invoke(inv)?,
                core::Command::Connect(con) => {
                    bind_check.connect(&con.dst, &con.src)?
                }
                core::Command::Fsm(_) | core::Command::Instance(_) => (),
            }
        }

        Ok(())
    }
}

pub fn check(mut ns: ast::Namespace) -> FilamentResult<ast::Namespace> {
    let comps = ns.components.drain(..).collect_vec();
    let sigs: HashMap<_, _> = ns.signatures().collect();
    for sig in sigs.values() {
        BindCheck::check_sig(sig)?;
    }
    let mut binds = Bindings::new(sigs);

    for comp in comps {
        BindCheck::component(&comp, &binds)?;
        binds.add_component(comp);
    }

    Ok(ast::Namespace {
        components: binds.into(),
        imports: ns.imports,
        externs: ns.externs,
    })
}
