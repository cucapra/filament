use crate::cmdline;
use fil_ir::{self as ir, Ctx, MutCtx};

#[must_use]
#[derive(PartialEq, Eq)]
/// Action performed by the visitor
pub enum Action {
    /// Stop visiting the CFG
    Stop,
    /// Continue visiting the CFG
    Continue,
    /// Add commands after this command
    AddBefore(Vec<ir::Command>),
    /// Change the current command with other commands
    Change(Vec<ir::Command>),
}

impl Action {
    /// Run the traversal specified by `next` if this traversal succeeds.
    /// If the result of this traversal is not `Action::Continue`, do not
    /// run `next()`.
    pub fn and_then<F>(self, mut next: F) -> Action
    where
        F: FnMut() -> Action,
    {
        match self {
            Action::Continue => next(),
            Action::Change(_) | Action::AddBefore(_) | Action::Stop => self,
        }
    }
}

/// Contains information passed to visitor functions.
/// This is re-generated when beginning to visit each component.
pub struct VisitorData<'comp> {
    /// The current component being visited
    pub comp: ir::Component,
    /// The idx of the current component.
    pub idx: ir::CompIdx,
    /// The command line options for this pass
    pub opts: &'comp cmdline::Opts,
    /// mutable context reference, held to prevent another
    /// function from mutating the context as it is currently invalid.
    pub mut_ctx: &'comp mut ir::Context,
}

impl<'comp> VisitorData<'comp> {
    /// Get an immutable reference to the current [ir::Context].
    pub fn ctx(&'comp self) -> &'comp ir::Context {
        self.mut_ctx
    }
    /// Get an immutable reference to a component.
    /// This is necessary because if the component being borrowed
    /// is exactly the same as the one being visited,
    /// the context no longer contains it.
    pub fn get(&self, idx: ir::CompIdx) -> &ir::Component {
        if idx == self.idx {
            &self.comp
        } else {
            self.mut_ctx.get(idx)
        }
    }
}

impl<'comp> From<(ir::CompIdx, &'comp cmdline::Opts, &'comp mut ir::Context)>
    for VisitorData<'comp>
{
    fn from(
        (idx, opts, ctx): (
            ir::CompIdx,
            &'comp cmdline::Opts,
            &'comp mut ir::Context,
        ),
    ) -> Self {
        let comp = std::mem::take(ctx.get_mut(idx));
        Self {
            comp,
            idx,
            opts,
            mut_ctx: ctx,
        }
    }
}

impl Drop for VisitorData<'_> {
    fn drop(&mut self) {
        // swaps the component back into the context when this visitordata is no longer used.
        std::mem::swap(self.mut_ctx.get_mut(self.idx), &mut self.comp);
    }
}

/// Construct a visitor
pub trait Construct {
    fn from(opts: &cmdline::Opts, ctx: &mut ir::Context) -> Self;

    /// Clear data before the next component has been visited
    fn clear_data(&mut self);
}

impl<T: Default> Construct for T {
    fn from(_: &cmdline::Opts, _: &mut ir::Context) -> Self {
        Self::default()
    }

    fn clear_data(&mut self) {
        *self = Self::default();
    }
}

/// A visitor for the commands by detaching them from the underlying component.
pub trait Visitor
where
    Self: Sized + Construct,
{
    /// The user visible name for the pass
    fn name() -> &'static str;

    #[must_use]
    /// Executed after the visitor has visited all the components.
    /// If the return value is `Some`, the number is treated as an error code.
    fn after_traversal(&mut self) -> Option<u64> {
        None
    }

    /// Executed before the visitor starts visiting the commands.
    /// The commands are still attached to the component
    fn start(&mut self, _data: &mut VisitorData) -> Action {
        Action::Continue
    }

    /// Executed after the visitor has visited all the commands.
    fn end(&mut self, _data: &mut VisitorData) {}

    fn invoke(&mut self, _: ir::InvIdx, _data: &mut VisitorData) -> Action {
        Action::Continue
    }

    fn instance(&mut self, _: ir::InstIdx, _data: &mut VisitorData) -> Action {
        Action::Continue
    }

    fn connect(
        &mut self,
        _: &mut ir::Connect,
        _data: &mut VisitorData,
    ) -> Action {
        Action::Continue
    }

    /// Executed before the body of the loop is visited
    fn start_loop(
        &mut self,
        _: &mut ir::Loop,
        _data: &mut VisitorData,
    ) -> Action {
        Action::Continue
    }

    /// Executed after the body of the loop is visited
    fn end_loop(
        &mut self,
        _: &mut ir::Loop,
        _data: &mut VisitorData,
    ) -> Action {
        Action::Continue
    }
    /// Traverse for `for` loops.
    /// Overriding this requires explicit traversal over the body.
    fn do_loop(&mut self, l: &mut ir::Loop, data: &mut VisitorData) -> Action {
        self.start_loop(l, data)
            .and_then(|| self.visit_cmds(&mut l.body, data))
            .and_then(|| self.end_loop(l, data))
    }

    /// Executed before the branches of the if is visited
    fn start_if(&mut self, _: &mut ir::If, _data: &mut VisitorData) -> Action {
        Action::Continue
    }
    /// Executed after the branches of the if is visited
    fn end_if(&mut self, _: &mut ir::If, _data: &mut VisitorData) -> Action {
        Action::Continue
    }
    /// Traverse for `if` statements.
    /// Overriding this requires explicit traversal over the body.
    fn do_if(&mut self, i: &mut ir::If, data: &mut VisitorData) -> Action {
        self.start_if(i, data)
            .and_then(|| self.visit_cmds(&mut i.then, data))
            .and_then(|| self.visit_cmds(&mut i.alt, data))
            .and_then(|| self.end_if(i, data))
    }

    fn fact(&mut self, _: &mut ir::Fact, _data: &mut VisitorData) -> Action {
        Action::Continue
    }

    fn bundle_def(
        &mut self,
        _: ir::PortIdx,
        _data: &mut VisitorData,
    ) -> Action {
        Action::Continue
    }

    fn exists(
        &mut self,
        _: &mut ir::Exists,
        _data: &mut VisitorData,
    ) -> Action {
        Action::Continue
    }

    fn let_(&mut self, _: &mut ir::Let, _data: &mut VisitorData) -> Action {
        Action::Continue
    }

    fn visit_cmd(
        &mut self,
        cmd: &mut ir::Command,
        data: &mut VisitorData,
    ) -> Action {
        match cmd {
            ir::Command::Instance(idx) => self.instance(*idx, data),
            ir::Command::Invoke(idx) => self.invoke(*idx, data),
            ir::Command::BundleDef(idx) => self.bundle_def(*idx, data),
            ir::Command::Connect(con) => self.connect(con, data),
            ir::Command::ForLoop(l) => self.do_loop(l, data),
            ir::Command::If(i) => self.do_if(i, data),
            ir::Command::Fact(f) => self.fact(f, data),
            ir::Command::Exists(e) => self.exists(e, data),
            ir::Command::Let(l) => self.let_(l, data),
        }
    }

    /// Perform action before visiting a sequence of commands which represent a scope.
    fn start_cmds(
        &mut self,
        _: &mut Vec<ir::Command>,
        _data: &mut VisitorData,
    ) {
    }

    /// Perform action after visiting a sequence of commands representing a scope.
    fn end_cmds(&mut self, _: &mut Vec<ir::Command>, _data: &mut VisitorData) {}

    fn visit_cmds(
        &mut self,
        cmds: &mut Vec<ir::Command>,
        data: &mut VisitorData,
    ) -> Action {
        self.start_cmds(cmds, data);

        let cs = std::mem::take(cmds);
        let mut n_cmds = Vec::with_capacity(cs.len());
        let mut iter = cs.into_iter();

        let mut stopped = false;
        for mut cmd in iter.by_ref() {
            match self.visit_cmd(&mut cmd, data) {
                Action::Stop => {
                    stopped = true;
                    break;
                }
                Action::Continue => {
                    n_cmds.push(cmd);
                }
                Action::Change(cmds) => {
                    n_cmds.extend(cmds.into_iter());
                }
                Action::AddBefore(cmds) => {
                    n_cmds.extend(cmds.into_iter());
                    n_cmds.push(cmd);
                }
            }
        }
        n_cmds.extend(iter);
        *cmds = n_cmds;

        if stopped {
            Action::Stop
        } else {
            self.end_cmds(cmds, data);
            Action::Continue
        }
    }

    fn visit(&mut self, mut data: VisitorData) {
        let pre_cmds = match self.start(&mut data) {
            Action::Stop => return,
            Action::Continue => None,
            Action::AddBefore(cmds) => Some(cmds),
            Action::Change(_) => {
                unreachable!("start should not change IR")
            }
        };

        // Traverse the commands
        let mut cmds = std::mem::take(&mut data.comp.cmds);
        match self.visit_cmds(&mut cmds, &mut data) {
            Action::Stop | Action::Continue => (),
            Action::Change(_) | Action::AddBefore(_) => {
                unreachable!("visit_cmds should not attempt to change IR nodes")
            }
        }

        if let Some(pre_cmds) = pre_cmds {
            data.comp.cmds = pre_cmds;
        }
        data.comp.cmds.extend(cmds);

        self.end(&mut data);
    }

    /// Apply the pass to all components in the context
    fn do_pass(opts: &cmdline::Opts, ctx: &mut ir::Context) -> Result<(), u64> {
        let mut visitor = Self::from(opts, ctx);
        for idx in ctx.comps.idx_iter() {
            visitor.clear_data();
            log::trace!("{}: Visiting component {}", Self::name(), idx);
            visitor.visit((idx, opts, &mut *ctx).into());
        }
        // for a pass to be valid, we should validate the context after the pass
        ir::Validate::context(ctx);

        match visitor.after_traversal() {
            Some(n) => Err(n),
            None => Ok(()),
        }
    }
}
