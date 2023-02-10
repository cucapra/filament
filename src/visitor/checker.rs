use crate::{
    core::{self, TimeRep, WidthRep},
    errors::FilamentResult,
};

use super::{CompBinding, ProgBinding};

/// A pass that checks the AST and computes some information about it without modifying it.
pub trait Checker<T, W>
where
    T: TimeRep,
    W: WidthRep,
    Self: Sized,
{
    /// Construct a new instance of this pass
    fn new() -> Self;

    /// Clear any data that should be cleared between components
    fn clear_data(&mut self);

    #[inline]
    fn connect(
        &mut self,
        _: &core::Connect,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    #[inline]
    fn instance(
        &mut self,
        _: &core::Instance<W>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    #[inline]
    fn fsm(
        &mut self,
        _: &core::Fsm,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        _: &core::Invoke<T>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    #[inline]
    fn signature(
        &mut self,
        sig: &core::Signature<T, W>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        comp: &core::Component<T, W>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        comp: &core::Component<T, W>,
        ctx: &CompBinding<T, W>,
    ) -> FilamentResult<()> {
        Ok(())
    }

    /// Perform the component traversal
    fn component(
        &mut self,
        comp: &core::Component<T, W>,
        prog_ctx: &super::ProgBinding<T, W>,
    ) -> FilamentResult<()> {
        let ctx = &CompBinding::new(prog_ctx, comp);

        // Binding for instances
        self.enter_component(comp, ctx)?;
        comp.body.iter().try_for_each(|cmd| match cmd {
            crate::core::Command::Invoke(inv) => self.invoke(inv, ctx),
            crate::core::Command::Instance(inst) => self.instance(inst, ctx),
            crate::core::Command::Connect(con) => self.connect(con, ctx),
            crate::core::Command::Fsm(fsm) => self.fsm(fsm, ctx),
        })?;
        self.exit_component(comp, ctx)
    }

    fn check(ns: &core::Namespace<T, W>) -> FilamentResult<Self> {
        let prog_ctx = &ProgBinding::from(ns);
        let mut pass = Self::new();

        for comp in &ns.components {
            pass.clear_data();
            pass.component(comp, &prog_ctx)?;
        }

        Ok(pass)
    }
}
