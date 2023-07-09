use crate::{cmdline, ir};

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

/// Construct a visitor
pub trait Construct {
    fn from(opts: &cmdline::Opts, ctx: &ir::Context) -> Self;

    /// Clear data before the next component has been visited
    fn clear_data(&mut self);
}

impl<T: Default> Construct for T {
    fn from(_: &cmdline::Opts, _: &ir::Context) -> Self {
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

    /// Executed after the visitor has visited all the components.
    /// If the return value is `Some`, the number is treated as an error code.
    fn after_traversal(&mut self) -> Option<u32> {
        None
    }

    /// Executed before the visitor starts visiting the commands.
    /// The commands are still attached to the component
    fn start(&mut self, _comp: &mut ir::Component) -> Action {
        Action::Continue
    }

    /// Executed after the visitor has visited all the commands.
    fn end(&mut self, _: &mut ir::Component) {}

    fn invoke(&mut self, _: ir::InvIdx, _comp: &mut ir::Component) -> Action {
        Action::Continue
    }

    fn instance(
        &mut self,
        _: ir::InstIdx,
        _comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }

    fn connect(
        &mut self,
        _: &mut ir::Connect,
        _comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }

    /// Executed before the body of the loop is visited
    fn start_loop(
        &mut self,
        _: &mut ir::Loop,
        _comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }

    /// Executed after the body of the loop is visited
    fn end_loop(
        &mut self,
        _: &mut ir::Loop,
        _comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }
    /// Traverse for `for` loops.
    /// Overriding this requires explicit traversal over the body.
    fn do_loop(
        &mut self,
        l: &mut ir::Loop,
        comp: &mut ir::Component,
    ) -> Action {
        self.start_loop(l, comp)
            .and_then(|| self.visit_cmds(&mut l.body, comp))
            .and_then(|| self.end_loop(l, comp))
    }

    /// Executed before the branches of the if is visited
    fn start_if(
        &mut self,
        _: &mut ir::If,
        _comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }
    /// Executed after the branches of the if is visited
    fn end_if(&mut self, _: &mut ir::If, _comp: &mut ir::Component) -> Action {
        Action::Continue
    }
    /// Traverse for `if` statements.
    /// Overriding this requires explicit traversal over the body.
    fn do_if(&mut self, i: &mut ir::If, comp: &mut ir::Component) -> Action {
        self.start_if(i, comp)
            .and_then(|| self.visit_cmds(&mut i.then, comp))
            .and_then(|| self.visit_cmds(&mut i.alt, comp))
            .and_then(|| self.end_if(i, comp))
    }

    fn fact(&mut self, _: &mut ir::Fact, _comp: &mut ir::Component) -> Action {
        Action::Continue
    }

    fn param_let(
        &mut self,
        _: &mut ir::Let,
        _comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }

    fn visit_cmd(
        &mut self,
        cmd: &mut ir::Command,
        comp: &mut ir::Component,
    ) -> Action {
        match cmd {
            ir::Command::Instance(idx) => self.instance(*idx, comp),
            ir::Command::Invoke(idx) => self.invoke(*idx, comp),
            ir::Command::Connect(con) => self.connect(con, comp),
            ir::Command::ForLoop(l) => self.do_loop(l, comp),
            ir::Command::If(i) => self.do_if(i, comp),
            ir::Command::Fact(f) => self.fact(f, comp),
            ir::Command::Let(l) => self.param_let(l, comp),
        }
    }

    /// Perform action before visiting a sequence of commands which represent a scope.
    fn start_cmds(&mut self, _: &mut Vec<ir::Command>, _: &mut ir::Component) {}

    /// Perform action after visiting a sequence of commands representing a scope.
    fn end_cmds(&mut self, _: &mut Vec<ir::Command>, _: &mut ir::Component) {}

    fn visit_cmds(
        &mut self,
        cmds: &mut Vec<ir::Command>,
        comp: &mut ir::Component,
    ) -> Action {
        self.start_cmds(cmds, comp);

        let cs = std::mem::take(cmds);
        let mut n_cmds = Vec::with_capacity(cs.len());
        let mut iter = cs.into_iter();

        let mut stopped = false;
        for mut cmd in iter.by_ref() {
            match self.visit_cmd(&mut cmd, comp) {
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
            self.end_cmds(cmds, comp);
            Action::Continue
        }
    }

    fn visit(&mut self, comp: &mut ir::Component) {
        let pre_cmds = match self.start(comp) {
            Action::Stop => return,
            Action::Continue => None,
            Action::AddBefore(cmds) => Some(cmds),
            Action::Change(_) => {
                unreachable!("start should not change IR")
            }
        };

        // Traverse the commands
        let mut cmds = std::mem::take(&mut comp.cmds);
        match self.visit_cmds(&mut cmds, comp) {
            Action::Stop | Action::Continue => (),
            Action::Change(_) | Action::AddBefore(_) => {
                unreachable!("visit_cmds should not attempt to change IR nodes")
            }
        }
        if let Some(pre_cmds) = pre_cmds {
            comp.cmds = pre_cmds;
        }
        comp.cmds.extend(cmds);

        self.end(comp);
    }

    /// Apply the pass to all components in the context
    fn do_pass(opts: &cmdline::Opts, ctx: &mut ir::Context) -> Result<(), u32> {
        let mut visitor = Self::from(opts, ctx);
        for (_, c) in ctx.comps.iter_mut() {
            visitor.clear_data();
            visitor.visit(c);
        }
        match visitor.after_traversal() {
            Some(n) => Err(n),
            None => Ok(()),
        }
    }
}
