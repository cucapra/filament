use crate::binding::{CompBinding, ProgBinding};
use crate::{ast, cmdline, diagnostics};
use std::ops::ControlFlow;

/// Should the traversal continue or stop?
pub type Traverse = ControlFlow<(), ()>;

/// A pass that checks the AST and computes some information about it without modifying it.
pub trait Checker
where
    Self: Sized,
{
    /// Construct a new instance of this pass
    fn new(_opts: &cmdline::Opts, _ns: &ast::Namespace) -> Self;

    /// Clear any data that should be cleared between components
    fn clear_data(&mut self);

    /// Get the diagnostics for this pass
    fn diagnostics(&mut self) -> &mut diagnostics::Diagnostics;

    #[inline]
    fn connect(&mut self, _: &ast::Connect, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn instance(&mut self, _: &ast::Instance, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn forloop(&mut self, l: &ast::ForLoop, ctx: &CompBinding) -> Traverse {
        for cmd in &l.body {
            self.command(cmd, ctx)?;
        }
        Traverse::Continue(())
    }

    #[inline]
    fn if_(&mut self, l: &ast::If, ctx: &CompBinding) -> Traverse {
        for cmd in &l.then {
            self.command(cmd, ctx)?;
        }
        for cmd in &l.alt {
            self.command(cmd, ctx)?;
        }
        Traverse::Continue(())
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(&mut self, _: &ast::Invoke, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn bundle(
        &mut self,
        _is_port: bool,
        _: &ast::Bundle,
        _ctx: &CompBinding,
    ) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn port_let(&mut self, _: &ast::PortLet, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn param_let(&mut self, _: &ast::ParamLet, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn signature(&mut self, _: &ast::Signature) -> Traverse {
        Traverse::Continue(())
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        comp: &ast::Component,
        _ctx: &CompBinding,
    ) -> Traverse {
        for p in comp.sig.ports() {
            if let ast::PortDef::Bundle(b) = p.inner() {
                self.bundle(true, b, _ctx)?
            }
        }
        Traverse::Continue(())
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        _: &ast::Component,
        _ctx: &CompBinding,
    ) -> Traverse {
        Traverse::Continue(())
    }

    #[inline]
    fn fact(&mut self, _: &ast::Fact, _ctx: &CompBinding) -> Traverse {
        Traverse::Continue(())
    }

    fn command(&mut self, cmd: &ast::Command, ctx: &CompBinding) -> Traverse {
        match cmd {
            ast::Command::Fact(a) => self.fact(a, ctx),
            ast::Command::Bundle(bl) => self.bundle(false, bl, ctx),
            ast::Command::Invoke(inv) => self.invoke(inv, ctx),
            ast::Command::Instance(inst) => self.instance(inst, ctx),
            ast::Command::Connect(con) => self.connect(con, ctx),
            ast::Command::ForLoop(l) => self.forloop(l, ctx),
            ast::Command::If(i) => self.if_(i, ctx),
            ast::Command::PortLet(l) => self.port_let(l, ctx),
            ast::Command::ParamLet(l) => self.param_let(l, ctx),
        }
    }

    /// Check the component signature and perform the component traversal
    fn component(
        &mut self,
        comp: &ast::Component,
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
    fn external(&mut self, sig: &ast::Signature) -> Traverse {
        self.signature(sig)
    }

    fn check(
        opts: &cmdline::Opts,
        ns: &ast::Namespace,
        ctx: &ProgBinding,
    ) -> Result<Self, u64> {
        let mut pass = Self::new(opts, ns);

        for (_, ext) in ns.externals() {
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
