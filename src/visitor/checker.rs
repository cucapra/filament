use crate::{core, diagnostics, errors::FilamentResult};

use super::{CompBinding, ProgBinding};

/// A pass that checks the AST and computes some information about it without modifying it.
pub trait Checker
where
    Self: Sized,
{
    /// Construct a new instance of this pass
    fn new(_: &core::Namespace) -> FilamentResult<Self>;

    /// Clear any data that should be cleared between components
    fn clear_data(&mut self);

    /// Get the diagnostics for this pass
    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics;

    /// Check if this component should be traversed
    fn component_filter(&self, _: &core::Component) -> bool {
        true
    }

    #[inline]
    fn connect(
        &mut self,
        _: &core::Connect,
        _ctx: &CompBinding,
    ) -> FilamentResult<()> {
        Ok(())
    }

    #[inline]
    fn instance(
        &mut self,
        _: &core::Instance,
        _ctx: &CompBinding,
    ) -> FilamentResult<()> {
        Ok(())
    }

    #[inline]
    fn fsm(&mut self, _: &core::Fsm, _ctx: &CompBinding) -> FilamentResult<()> {
        Ok(())
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        _: &core::Invoke,
        _ctx: &CompBinding,
    ) -> FilamentResult<()> {
        Ok(())
    }

    #[inline]
    fn signature(&mut self, _: &core::Signature) -> FilamentResult<()> {
        Ok(())
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        _: &core::Component,
        _ctx: &CompBinding,
    ) -> FilamentResult<()> {
        Ok(())
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        _: &core::Component,
        _ctx: &CompBinding,
    ) -> FilamentResult<()> {
        Ok(())
    }

    /// Check the component signature and perform the component traversal
    fn component(
        &mut self,
        comp: &core::Component,
        prog_ctx: &super::ProgBinding,
    ) -> FilamentResult<()> {
        self.signature(&comp.sig)?;
        let ctx = &CompBinding::new(prog_ctx, comp)?;

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

    /// Check an external signature. By default, simply calls `signature`.
    fn external(&mut self, sig: &core::Signature) -> FilamentResult<()> {
        self.signature(sig)
    }

    fn check(ns: &core::Namespace) -> FilamentResult<Self> {
        let prog_ctx = &ProgBinding::from(ns);
        let mut pass = Self::new(ns)?;

        for (_, ext) in ns.signatures() {
            pass.external(ext)?;
        }

        for comp in &ns.components {
            if pass.component_filter(comp) {
                pass.clear_data();
                pass.component(comp, prog_ctx)?;
            }
        }

        Ok(pass)
    }
}
