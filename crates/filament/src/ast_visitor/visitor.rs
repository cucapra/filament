use crate::cmdline;
use fil_ast as ast;

#[must_use]
/// Action performed by the visitor
pub enum Action {
    /// Stop visiting the CFG
    Stop,
    /// Continue visiting the CFG
    Continue,
    /// Add commands after this command
    AddBefore(Vec<ast::Command>),
    /// Change the current command with other commands
    Change(Vec<ast::Command>),
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
    fn from(opts: &cmdline::Opts, ast: &mut ast::Namespace) -> Self;

    /// Clear data before the next component has been visited
    fn clear_data(&mut self);
}

impl<T: Default> Construct for T {
    fn from(_: &cmdline::Opts, _: &mut ast::Namespace) -> Self {
        Self::default()
    }

    fn clear_data(&mut self) {
        *self = Self::default();
    }
}

/// Visit and transform the given AST
pub trait Visitor
where
    Self: Sized + Construct,
{
    fn bundle(&mut self, _: &mut ast::Bundle) -> Action {
        Action::Continue
    }

    fn connect(&mut self, _: &mut ast::Connect) -> Action {
        Action::Continue
    }

    fn exists(&mut self, _: &mut ast::Exists) -> Action {
        Action::Continue
    }

    fn fact(&mut self, _: &mut ast::Fact) -> Action {
        Action::Continue
    }

    fn start_loop(&mut self, _: &mut ast::ForLoop) -> Action {
        Action::Continue
    }

    fn end_loop(&mut self, _: &mut ast::ForLoop) -> Action {
        Action::Continue
    }

    fn do_loop(&mut self, l: &mut ast::ForLoop) -> Action {
        self.start_loop(l)
            .and_then(|| self.visit_cmds(&mut l.body))
            .and_then(|| self.end_loop(l))
    }

    fn start_if(&mut self, _: &mut ast::If) -> Action {
        Action::Continue
    }

    fn end_if(&mut self, _: &mut ast::If) -> Action {
        Action::Continue
    }

    fn do_if(&mut self, i: &mut ast::If) -> Action {
        self.start_if(i)
            .and_then(|| self.visit_cmds(&mut i.then))
            .and_then(|| self.visit_cmds(&mut i.alt))
            .and_then(|| self.end_if(i))
    }

    fn instance(&mut self, _: &mut ast::Instance) -> Action {
        Action::Continue
    }

    fn invoke(&mut self, _: &mut ast::Invoke) -> Action {
        Action::Continue
    }

    fn param_let(&mut self, _: &mut ast::ParamLet) -> Action {
        Action::Continue
    }

    fn visit_cmd(&mut self, cmd: &mut ast::Command) -> Action {
        match cmd {
            ast::Command::Bundle(bundle) => self.bundle(bundle),
            ast::Command::Connect(connect) => self.connect(connect),
            ast::Command::Exists(exists) => self.exists(exists),
            ast::Command::Fact(fact) => self.fact(fact),
            ast::Command::ForLoop(forloop) => self.do_loop(forloop),
            ast::Command::If(i) => self.do_if(i),
            ast::Command::Instance(inst) => self.instance(inst),
            ast::Command::Invoke(inv) => self.invoke(inv),
            ast::Command::ParamLet(pl) => self.param_let(pl),
        }
    }

    fn start_cmds(&mut self, _: &mut Vec<ast::Command>) {}

    fn end_cmds(&mut self, _: &mut Vec<ast::Command>) {}

    /// Visit a list of commands (a scope)
    fn visit_cmds(&mut self, cmds: &mut Vec<ast::Command>) -> Action {
        self.start_cmds(cmds);

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
            self.end_cmds(cmds);
            Action::Continue
        }
    }

    /// Visit a component signature
    fn signature(&mut self, sig: &mut ast::Signature) {}

    /// Visit the AST
    fn visit(&mut self, comp: &mut ast::Component) {
        self.signature(&mut comp.sig);
        self.visit_cmds(&mut comp.body);
    }

    fn finish(self, ast: ast::Namespace) -> ast::Namespace;

    /// Perform the pass
    fn do_pass(
        opts: &cmdline::Opts,
        mut ast: ast::Namespace,
    ) -> Result<ast::Namespace, u64> {
        let mut visitor = Self::from(opts, &mut ast);
        for comp in ast.components.iter_mut() {
            visitor.visit(comp);
            visitor.clear_data();
        }

        for ext in ast.externs.iter_mut() {}
        Ok(visitor.finish(ast))
    }
}
