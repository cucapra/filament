use crate::binding::{BoundComponent, CompBinding, ProgBinding};
use crate::{ast, errors::FilamentResult};
use itertools::Itertools;

/// Transform the given AST
pub trait Transform
where
    Self: Sized,
{
    /// Extra information needed to construct this visitor.
    type Info;

    /// Construct an instance of this pass
    fn new(_: &ast::Namespace, info: &Self::Info) -> Self;

    /// What data should be cleared between component
    fn clear_data(&mut self);

    #[inline]
    fn connect(
        &mut self,
        con: ast::Connect,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![con.into()])
    }

    #[inline]
    fn instance(
        &mut self,
        inst: ast::Instance,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![inst.into()])
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![inv.into()])
    }

    #[inline]
    fn signature(
        &mut self,
        sig: ast::Signature,
        _: &CompBinding,
    ) -> FilamentResult<ast::Signature> {
        Ok(sig)
    }

    #[inline]
    fn bundle(
        &mut self,
        bundle: ast::Bundle,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![bundle.into()])
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![])
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        _: &CompBinding,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![])
    }

    /// Generate FSMs to be added to the component. This is called after the
    /// component traversal.
    #[inline]
    fn fsms(&mut self) -> FilamentResult<Vec<ast::Fsm>> {
        Ok(vec![])
    }

    /// Transform the program
    fn transform(
        mut ns: ast::Namespace,
        info: Self::Info,
    ) -> FilamentResult<(ast::Namespace, Self)> {
        // Build a new pass
        let mut pass = Self::new(&ns, &info);

        // Extract (name, commands) from the components
        let comp_data: Vec<(ast::Id, Vec<ast::Command>)> = ns
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

            // Traverse over the commands and apply the transfomation functions
            let mut n_cmds = Vec::with_capacity(cmds.len());
            n_cmds.extend(pass.enter_component(&ctx)?);

            for cmd in cmds {
                let cmds = match cmd {
                    ast::Command::Invoke(inv) => pass.invoke(inv, &ctx)?,
                    ast::Command::Instance(inst) => {
                        pass.instance(inst, &ctx)?
                    }
                    ast::Command::Connect(con) => pass.connect(con, &ctx)?,
                    ast::Command::Bundle(bl) => pass.bundle(bl, &ctx)?,
                    ast::Command::Fact(_) => unreachable!(
                        "Visitor does not support transforming assumptions"
                    ),
                    ast::Command::ForLoop(_) => unreachable!(
                        "Visitor does not support transforming loops"
                    ),
                    ast::Command::If(_) => unreachable!(
                        "Visitor does not support transforming if statements"
                    ),
                };
                n_cmds.extend(cmds);
            }

            let cmds = pass.exit_component(&ctx)?;
            n_cmds.extend(cmds);
            let fsms = pass.fsms()?;
            new_comp_data.push((n_cmds, fsms));
        }

        // Add the updated commands to the components
        for (comp, (cmds, fsms)) in ns.components.iter_mut().zip(new_comp_data)
        {
            comp.body = cmds;
            comp.fsms.extend(fsms);
        }

        Ok((ns, pass))
    }

    /// Report the error message if any occur during the transformation
    fn transform_unwrap(
        ns: ast::Namespace,
        info: Self::Info,
    ) -> Option<ast::Namespace> {
        match Self::transform(ns, info) {
            Ok((ns, _)) => Some(ns),
            Err(err) => {
                eprintln!("Error: {}", err.kind);
                None
            }
        }
    }
}
