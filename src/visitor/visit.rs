use crate::binding::{BoundComponent, CompBinding, ProgBinding};
use crate::{core, errors::FilamentResult};
use itertools::Itertools;

/// Transform the given AST
pub trait Transform
where
    Self: Sized,
{
    /// Extra information needed to construct this visitor.
    type Info;

    /// Construct an instance of this pass
    fn new(_: &core::Namespace, info: &Self::Info) -> Self;

    /// What data should be cleared between component
    fn clear_data(&mut self);

    /// Whether this component should be visited or not
    fn component_filter(&self, comp: &CompBinding) -> bool;

    #[inline]
    fn connect(
        &mut self,
        con: core::Connect,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![con.into()])
    }

    #[inline]
    fn instance(
        &mut self,
        inst: core::Instance,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![inst.into()])
    }

    #[inline]
    fn fsm(
        &mut self,
        fsm: core::Fsm,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![fsm.into()])
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        inv: core::Invoke,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![inv.into()])
    }

    #[inline]
    fn signature(
        &mut self,
        sig: core::Signature,
        _: &CompBinding,
    ) -> FilamentResult<core::Signature> {
        Ok(sig)
    }

    #[inline]
    fn bundle(
        &mut self,
        bundle: core::Bundle,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![bundle.into()])
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![])
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        _: &CompBinding,
    ) -> FilamentResult<Vec<core::Command>> {
        Ok(vec![])
    }

    /// Transform the program
    fn transform(
        mut ns: core::Namespace,
        info: Self::Info,
    ) -> FilamentResult<(core::Namespace, Self)> {
        // Build a new pass
        let mut pass = Self::new(&ns, &info);

        // Extract (name, commands) from the components
        let comp_data: Vec<(core::Id, Vec<core::Command>)> = ns
            .components
            .iter_mut()
            .map(|comp| {
                (*comp.sig.name.inner(), comp.body.drain(..).collect_vec())
            })
            .collect_vec();

        // Collect the updated component data
        let mut new_comp_data = Vec::with_capacity(comp_data.len());

        // The program binding
        let prog_bind = ProgBinding::try_from(&ns)
            .unwrap_or_else(|_| panic!("Failed to create a valid binding"));
        for (name, cmds) in comp_data {
            pass.clear_data();
            // Manually construct the component binding because we removed all
            // commands from components previously.
            let bind = BoundComponent::from_component(&prog_bind, &name, &cmds);
            let ctx = CompBinding {
                prog: &prog_bind,
                comp: &bind,
            };
            if !pass.component_filter(&ctx) {
                new_comp_data.push(cmds);
                continue;
            }

            // Traverse over the commands and apply the transfomation functions
            let mut n_cmds = Vec::with_capacity(cmds.len());
            n_cmds.extend(pass.enter_component(&ctx)?);

            for cmd in cmds {
                let cmds = match cmd {
                    core::Command::Invoke(inv) => pass.invoke(inv, &ctx)?,
                    core::Command::Instance(inst) => {
                        pass.instance(inst, &ctx)?
                    }
                    core::Command::Connect(con) => pass.connect(con, &ctx)?,
                    core::Command::Fsm(fsm) => pass.fsm(fsm, &ctx)?,
                    core::Command::Bundle(bl) => pass.bundle(bl, &ctx)?,
                    core::Command::ForLoop(_) => unreachable!(
                        "Visitor does not support transforming loops"
                    ),
                    core::Command::If(_) => unreachable!(
                        "Visitor does not support transforming if statements"
                    ),
                };
                n_cmds.extend(cmds);
            }

            let cmds = pass.exit_component(&ctx)?;
            n_cmds.extend(cmds);
            new_comp_data.push(n_cmds);
        }

        // Add the updated commands to the components
        for (comp, cmds) in ns.components.iter_mut().zip(new_comp_data) {
            comp.body = cmds;
        }

        Ok((ns, pass))
    }

    /// Report the error message if any occur during the transformation
    fn transform_unwrap(
        ns: core::Namespace,
        info: Self::Info,
    ) -> Option<core::Namespace> {
        match Self::transform(ns, info) {
            Ok((ns, _)) => Some(ns),
            Err(err) => {
                eprintln!("Error: {}", err.kind);
                None
            }
        }
    }
}
