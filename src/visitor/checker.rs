use std::ops::ControlFlow;

use crate::binding::{CompBinding, ProgBinding};
use crate::{core, diagnostics};

/// Should the traversal continue or stop?
pub type Traverse = ControlFlow<(), ()>;

/// A pass that checks the AST and computes some information about it without modifying it.
pub trait Checker
where
    Self: Sized,
{
    /// Construct a new instance of this pass
    fn new(_: &core::Namespace) -> Self;

    /// Clear any data that should be cleared between components
    fn clear_data(&mut self);

    /// Get the diagnostics for this pass
    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics;

    #[inline]
    fn connect(&mut self, _: &core::Connect, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn instance(&mut self, _: &core::Instance, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn fsm(&mut self, _: &core::Fsm, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn forloop(&mut self, l: &core::ForLoop, ctx: &CompBinding) -> Traverse {
        for cmd in &l.body {
            self.command(cmd, ctx)?;
        }
        Traverse::Continue(())
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(&mut self, _: &core::Invoke, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn signature(&mut self, _: &core::Signature) -> Traverse {
        Traverse::Continue(())
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        _: &core::Component,
        _ctx: &CompBinding,
    ) -> Traverse {
        Traverse::Continue(())
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        _: &core::Component,
        _ctx: &CompBinding,
    ) -> Traverse {
        Traverse::Continue(())
    }

    fn command(&mut self, cmd: &core::Command, ctx: &CompBinding) -> Traverse {
        match cmd {
            core::Command::Invoke(inv) => self.invoke(inv, ctx),
            core::Command::Instance(inst) => self.instance(inst, ctx),
            core::Command::Connect(con) => self.connect(con, ctx),
            core::Command::Fsm(fsm) => self.fsm(fsm, ctx),
            core::Command::ForLoop(l) => self.forloop(l, ctx),
        }
    }

    /// Check the component signature and perform the component traversal
    fn component(
        &mut self,
        comp: &core::Component,
        prog_ctx: &ProgBinding,
    ) -> Traverse {
        self.signature(&comp.sig)?;
        let ctx = &CompBinding::new(prog_ctx, &comp.sig.name);
        // Binding for instances
        self.enter_component(comp, ctx)?;
        comp.body
            .iter()
            .try_for_each(|cmd| self.command(cmd, ctx))?;
        self.exit_component(comp, ctx)
    }

    /// Check an external signature. By default, simply calls `signature`.
    fn external(&mut self, sig: &core::Signature) -> Traverse {
        self.signature(sig)
    }

    fn check(ns: &core::Namespace, ctx: &ProgBinding) -> Result<Self, u64> {
        let mut pass = Self::new(ns);

        for (_, ext) in ns.signatures() {
            // Ignore the return value from traversal because we don't need to
            // abort anything from it.
            pass.external(ext);
        }

        for comp in &ns.components {
            pass.clear_data();
            pass.component(comp, ctx);
        }

        if let Some(errs) = pass.diagnostics().report_all() {
            Err(errs)
        } else {
            Ok(pass)
        }
    }
}
