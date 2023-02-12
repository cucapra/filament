use super::{CompBinding, ProgBinding};
use crate::{
    core::{self, TimeRep, WidthRep},
    errors::FilamentResult,
};
use itertools::Itertools;

/// Transform the given AST
pub trait Transform<T, W>
where
    Self: Sized,
    T: TimeRep,
    W: WidthRep,
{
    /// Extra information needed to construct this visitor.
    type Info;

    /// Construct an instance of this pass
    fn new(_: &core::Namespace<T, W>, info: &Self::Info) -> Self;

    /// What data should be cleared between component
    fn clear_data(&mut self);

    /// Whether this component should be visited or not
    fn component_filter(&self, comp: &CompBinding<T, W>) -> bool;

    #[inline]
    fn connect(
        &mut self,
        con: core::Connect,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![con.into()])
    }

    #[inline]
    fn instance(
        &mut self,
        inst: core::Instance<W>,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![inst.into()])
    }

    #[inline]
    fn fsm(
        &mut self,
        fsm: core::Fsm,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![fsm.into()])
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        inv: core::Invoke<T>,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![inv.into()])
    }

    #[inline]
    fn signature(
        &mut self,
        sig: core::Signature<T, W>,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<core::Signature<T, W>> {
        Ok(sig)
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![])
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        _: &CompBinding<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![])
    }

    /// Transform the program
    fn transform(
        mut ns: core::Namespace<T, W>,
        info: Self::Info,
    ) -> FilamentResult<(core::Namespace<T, W>, Self)> {
        // Build a new pass
        let mut pass = Self::new(&ns, &info);

        // Extract (name, commands) from the components
        let comp_data: Vec<(core::Id, Vec<core::Command<T, W>>)> = ns
            .components
            .iter_mut()
            .map(|comp| {
                (comp.sig.name.clone(), comp.body.drain(..).collect_vec())
            })
            .collect_vec();

        // Collect the updated component data
        let mut new_comp_data = Vec::with_capacity(comp_data.len());

        // The program binding
        let prog_bind = ProgBinding::from(&ns);
        for (name, cmds) in comp_data {
            pass.clear_data();
            let ctx = CompBinding::from_comp_data(&prog_bind, &name, &cmds)?;
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
}
