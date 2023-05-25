use crate::ir::{self, Ctx};

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

/// A visitor for the commands by detaching them from the underlying component.
pub trait Visitor {
    /// Executed before the visitor starts visiting the commands.
    /// The commands are still attached to the component
    fn start(&mut self, comp: &mut ir::Component) {}

    fn end(&mut self) {}

    fn invoke(&mut self, _: ir::InvIdx, comp: &mut ir::Component) -> Action {
        Action::Continue
    }

    fn instance(&mut self, _: ir::InstIdx, comp: &mut ir::Component) -> Action {
        Action::Continue
    }

    fn connect(
        &mut self,
        _: &mut ir::Connect,
        comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }

    /// Executed before the body of the loop is visited
    fn start_loop(
        &mut self,
        _: &mut ir::Loop,
        comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }
    /// Executed after the body of the loop is visited
    fn end_loop(
        &mut self,
        _: &mut ir::Loop,
        comp: &mut ir::Component,
    ) -> Action {
        Action::Continue
    }

    /// Executed before the branches of the if is visited
    fn start_if(&mut self, _: &mut ir::If, comp: &mut ir::Component) -> Action {
        Action::Continue
    }
    /// Executed after the branches of the if is visited
    fn end_if(&mut self, _: &mut ir::If, comp: &mut ir::Component) -> Action {
        Action::Continue
    }

    fn fact(&mut self, _: &mut ir::Fact, comp: &mut ir::Component) -> Action {
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
            ir::Command::ForLoop(l) => self
                .start_loop(l, comp)
                .and_then(|| self.visit_cmds(&mut l.body, comp))
                .and_then(|| self.end_loop(l, comp)),
            ir::Command::If(i) => self
                .start_if(i, comp)
                .and_then(|| self.visit_cmds(&mut i.then, comp))
                .and_then(|| self.visit_cmds(&mut i.alt, comp))
                .and_then(|| self.end_if(i, comp)),
            ir::Command::Fact(f) => self.fact(f, comp),
        }
    }

    fn visit_cmds(
        &mut self,
        cmds: &mut Vec<ir::Command>,
        comp: &mut ir::Component,
    ) -> Action {
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

    fn visit(&mut self, comp: &mut ir::Component) {
        self.start(comp);

        // Traverse the commands
        let mut cmds = std::mem::take(&mut comp.cmds);
        match self.visit_cmds(&mut cmds, comp) {
            Action::Stop | Action::Continue => (),
            Action::Change(_) => {
                unreachable!("visit_cmds should not attempt to change IR nodes")
            }
        }
        comp.cmds = cmds;

        self.end();
    }
}

/// Hoist all [ir::Fact] from the control flow graph into the top level by
/// adding their path conditions.
pub struct HoistFacts<'a> {
    comp: &'a mut ir::Component,
    /// Stack of path conditions
    stack: Vec<usize>,
    /// The current path condition
    path_cond: Vec<ir::PropIdx>,
}

impl<'a> From<&'a mut ir::Component> for HoistFacts<'a> {
    fn from(comp: &'a mut ir::Component) -> Self {
        Self {
            comp,
            stack: vec![],
            path_cond: vec![],
        }
    }
}

impl HoistFacts<'_> {
    /// Push a new stack frame by tracking the number of added path conditions
    pub fn push(&mut self) {
        let len = self.path_cond.len();
        let props = if let Some(prev) = self.stack.last() {
            len - prev
        } else {
            len
        };
        self.stack.push(props);
    }

    /// Pop the current stack frame
    pub fn pop(&mut self) {
        let props = self.stack.pop().unwrap();
        self.path_cond.truncate(self.path_cond.len() - props);
    }

    /// Insert a new path condition
    pub fn insert(&mut self, prop: ir::PropIdx) {
        self.path_cond.push(prop);
    }

    /// Return the current path condition
    pub fn path_cond(&mut self, comp: &mut ir::Component) -> ir::PropIdx {
        let mut pc = comp.add(ir::Prop::True);
        for prop in self.path_cond.iter().copied() {
            pc = pc.and(prop, comp);
        }
        pc
    }
}

impl Visitor for HoistFacts<'_> {}
