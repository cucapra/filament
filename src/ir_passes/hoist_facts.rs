use crate::ir;

#[must_use]
/// Action performed by the visitor
pub enum Action {
    /// Stop visiting the CFG
    Stop,
    /// Continue visiting the CFG
    Continue,
    /// Change the current command with other commands
    Change(Vec<ir::Command>),
}

impl Action {
    /// Run the traversal specified by `next` if this traversal succeeds.
    /// If the result of this traversal is not `Action::Continue`, do not
    /// run `next()`.
    pub(super) fn and_then<F>(self, mut next: F) -> Action
    where
        F: FnMut() -> Action,
    {
        match self {
            Action::Continue => next(),
            Action::Change(_) | Action::Stop => self,
        }
    }
}

/// A pass over the intermediate representation.
///
/// The pass must take ownership of the component and provide mutable access to it.
pub trait IRPass {
    /// Get a mutable reference to the component
    fn comp_mut(&mut self) -> &mut ir::Component;
}

/// A visitor for the commands by detaching them from the underlying component.
pub trait Visitor
where
    Self: IRPass + Sized,
{
    /// Executed before the visitor starts visiting the commands.
    /// The commands are still attached to the component
    fn start(&mut self) {}

    fn end(&mut self) {}

    fn invoke(&mut self, _: ir::InvIdx) -> Action {
        Action::Continue
    }

    fn instance(&mut self, _: ir::InstIdx) -> Action {
        Action::Continue
    }

    fn connect(&mut self, _: &mut ir::Connect) -> Action {
        Action::Continue
    }

    /// Executed before the body of the loop is visited
    fn start_loop(&mut self, _: &mut ir::Loop) -> Action {
        Action::Continue
    }
    /// Executed after the body of the loop is visited
    fn end_loop(&mut self, _: &mut ir::Loop) -> Action {
        Action::Continue
    }

    /// Executed before the branches of the if is visited
    fn start_if(&mut self, _: &mut ir::If) -> Action {
        Action::Continue
    }
    /// Executed after the branches of the if is visited
    fn end_if(&mut self, _: &mut ir::If) -> Action {
        Action::Continue
    }

    fn fact(&mut self, _: &mut ir::Fact) -> Action {
        Action::Continue
    }

    fn visit_cmd(&mut self, cmd: &mut ir::Command) -> Action {
        match cmd {
            ir::Command::Instance(idx) => self.instance(*idx),
            ir::Command::Invoke(idx) => self.invoke(*idx),
            ir::Command::Connect(con) => self.connect(con),
            ir::Command::ForLoop(l) => self
                .start_loop(l)
                .and_then(|| self.visit_cmds(&mut l.body))
                .and_then(|| self.end_loop(l)),
            ir::Command::If(i) => self
                .start_if(i)
                .and_then(|| self.visit_cmds(&mut i.then))
                .and_then(|| self.visit_cmds(&mut i.alt))
                .and_then(|| self.end_if(i)),
            ir::Command::Fact(f) => self.fact(f),
        }
    }

    fn visit_cmds(&mut self, cmds: &mut Vec<ir::Command>) -> Action {
        let cs = std::mem::take(cmds);
        let mut n_cmds = Vec::with_capacity(cs.len());
        let mut iter = cs.into_iter();

        let mut stopped = false;
        for mut cmd in iter.by_ref() {
            match self.visit_cmd(&mut cmd) {
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
            }
        }
        n_cmds.extend(iter);
        *cmds = n_cmds;

        if stopped {
            Action::Stop
        } else {
            Action::Continue
        }
    }

    fn visit(&mut self) {
        self.start();

        // Traverse the commands
        let mut cmds = std::mem::take(&mut self.comp_mut().cmds);
        match self.visit_cmds(&mut cmds) {
            Action::Stop | Action::Continue => (),
            Action::Change(_) => {
                unreachable!("visit_cmds should not attempt to change IR nodes")
            }
        }
        self.comp_mut().cmds = cmds;

        self.end();
    }
}

/// Hoist all [ir::Fact] from the control flow graph into the top level by
/// adding their path conditions.
pub struct HoistFacts;
