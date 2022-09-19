use crate::{
    bind_map::BindMap,
    errors::{Error, FilamentResult, WithPos},
    event_checker::ast,
    visitor::{Bindings, ResolvedInstance},
};
use itertools::Itertools;

const THIS: &str = "_this";

struct BindCheck<'a> {
    instances: BindMap<ResolvedInstance<'a>>,
    // mapping from name of invocations to the instance that they invoke
    invocations: BindMap<ast::Id>,
}

/// Transform the givun AST
impl BindCheck<'_> {
    fn new() -> Self {
        Self {
            instances: BindMap::new(),
            invocations: BindMap::new(),
        }
    }

    // Checks if the port is well defined and returns its width.
    fn port<const INPUT: bool>(
        &mut self,
        port: &ast::Port,
    ) -> FilamentResult<u64> {
        let check_port =
            |instance: &ast::Id, p: &ast::Id| -> FilamentResult<u64> {
                let sig = self
                    .instances
                    .get(instance)
                    .expect("THIS component is not defined")
                    .resolve();
                let mut iter = if INPUT {
                    sig.inputs.iter()
                } else {
                    sig.outputs.iter()
                };
                let kind = if INPUT { "input" } else { "output" };
                iter.find(|p1| p1.name == p)
                    .map(|p| p.bitwidth)
                    // XXX(rachit): Always search interface ports regardless of input or output because we don't
                    // correctly reverse them.
                    .or_else(|| {
                        sig.interface_signals
                            .iter()
                            .find(|def| def.name == p)
                            .map(|_| 1)
                    })
                    .ok_or_else(|| {
                        Error::undefined(ast::Id::from(format!("{port}")), kind)
                            .add_note("Port is not defined", port.copy_span())
                    })
            };

        match &port.typ {
            ast::PortType::ThisPort(p) => check_port(&ast::Id::from(THIS), p),
            ast::PortType::InvPort { invoke, name } => {
                let inst = self.invocations.get(invoke).map_err(|err| {
                    err.add_note(
                        "Invocation is not defined",
                        invoke.copy_span(),
                    )
                })?;
                check_port(inst, name)
            }
            ast::PortType::Constant(_) => Ok(32),
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
                    format!("Source {} has width {src_w}", src.name()),
                    src.copy_span(),
                )
                .add_note(
                    format!("Destination {} has width {dst_w}", dst.name(),),
                    dst.copy_span(),
                ));
        }
        Ok(())
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(&mut self, inv: &ast::Invoke) -> FilamentResult<()> {
        self.invocations
            .add(inv.bind.clone(), inv.instance.clone())?;
        // Get the signature for the instance
        let inst = self.instances.get(&inv.instance)?;

        // Check that the number of arguments matches the number of parameters
        let formals = inst.abstract_vars().len();
        let actuals = inv.abstract_vars.len();
        if formals != actuals {
            return Err(Error::malformed(format!(
                "Invoke of {} requires {formals} events but {actuals} are provided",
                inv.instance,
            )).add_note(
                format!("Invoke requires {formals} events but {actuals} are provided"),
                inv.instance.copy_span()
            ));
        }

        if let Some(ports) = &inv.ports {
            // Check that the number of ports matches the number of ports
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
            let sig = inst.resolve();
            for (actual, formal) in ports.iter().zip(sig.inputs.iter()) {
                let dst =
                    ast::Port::comp(inv.bind.clone(), formal.name.clone())
                        .set_span(formal.copy_span());
                self.connect(&dst, actual)?;
            }
        }

        Ok(())
    }

    /// Perform the component traversal
    fn component(
        comp: &ast::Component,
        binds: &Bindings,
    ) -> FilamentResult<()> {
        // Binding for instances
        let mut bind_check = BindCheck::new();

        // Add THIS instance
        let this_sig = comp.sig.reversed();
        bind_check
            .instances
            .add(ast::Id::from(THIS), (&this_sig).into())?;

        for cmd in &comp.body {
            match cmd {
                crate::core::Command::Invoke(inv) => bind_check.invoke(inv)?,
                crate::core::Command::Instance(inst) => {
                    let sig = binds
                        .find_component(&inst.component, &inst.bindings)?;
                    bind_check.instances.add(inst.name.clone(), sig)?;
                }
                crate::core::Command::Connect(con) => {
                    bind_check.connect(&con.dst, &con.src)?
                }
                crate::core::Command::Fsm(_) => (),
            }
        }

        Ok(())
    }
}

pub fn check(mut ns: ast::Namespace) -> FilamentResult<ast::Namespace> {
    let comps = ns.components.drain(..).collect_vec();
    let mut binds = Bindings::new(ns.signatures());

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
