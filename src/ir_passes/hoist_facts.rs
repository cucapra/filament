use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, AddCtx};

#[derive(Default)]
/// Hoist all [ir::Fact] from the control flow graph into the top level by
/// adding their path conditions.
pub struct HoistFacts {
    /// Stack of path conditions
    stack: Vec<usize>,
    /// The current path condition
    path_cond: Vec<ir::PropIdx>,
    /// Facts to be hoisted
    facts: Vec<ir::Command>,
}

impl HoistFacts {
    /// Push a new stack frame by tracking the number of added path conditions
    fn push(&mut self) {
        let len = self.path_cond.len();
        self.stack.push(len);
    }

    /// Pop the current stack frame
    fn pop(&mut self) {
        let props = self.stack.pop().unwrap();
        self.path_cond.truncate(props);
    }

    /// Insert a new path condition
    fn add_to_pc(&mut self, prop: ir::PropIdx) {
        self.path_cond.push(prop);
    }

    /// Return the current path condition
    fn path_cond(&mut self, comp: &mut ir::Component) -> ir::PropIdx {
        let mut pc = comp.add(ir::Prop::True);
        for prop in self.path_cond.iter().copied() {
            pc = pc.and(prop, comp);
        }
        pc
    }
}

impl Visitor for HoistFacts {
    fn name() -> &'static str {
        "hoist-facts"
    }

    /// Collect all assumptions in a given scope and add them to the path condition.
    /// We do this so that all asserts in a scope are affected by all assumes.
    fn start_cmds(
        &mut self,
        cmds: &mut Vec<ir::Command>,
        data: &mut VisitorData,
    ) {
        let ctx = &mut data.comp;
        cmds.iter().for_each(|cmd| match cmd {
            ir::Command::Fact(fact) if fact.is_assume() => {
                self.add_to_pc(fact.prop)
            }
            ir::Command::Exists(ir::Exists { param, expr }) => {
                let prop = param.expr(ctx).equal(*expr, ctx);
                self.add_to_pc(prop);
            }
            _ => (),
        });
    }

    fn fact(&mut self, fact: &mut ir::Fact, data: &mut VisitorData) -> Action {
        if fact.is_assert() {
            // Otherwise this is a checked assertion that needs to be hoisted.
            // Generate prop = path_cond -> fact.prop
            let cond = self
                .path_cond(&mut data.comp)
                .implies(fact.prop, &mut data.comp);
            self.facts.extend(data.comp.assert(cond, fact.reason));
        }
        Action::Change(vec![])
    }

    fn do_if(&mut self, i: &mut ir::If, data: &mut VisitorData) -> Action {
        self.push();
        self.add_to_pc(i.cond);
        let ac = self.visit_cmds(&mut i.then, data);
        assert!(ac == Action::Continue);
        self.pop();

        self.push();
        self.add_to_pc(i.cond.not(&mut data.comp));
        let ac = self.visit_cmds(&mut i.alt, data);
        assert!(ac == Action::Continue);
        self.pop();

        Action::Continue
    }

    fn start_loop(
        &mut self,
        l: &mut ir::Loop,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &mut data.comp;
        self.push();
        let ir::Loop {
            index, start, end, ..
        } = l;
        let idx = index.expr(comp);
        let start = idx.gte(*start, comp);
        let end = idx.lt(*end, comp);
        self.add_to_pc(start.and(end, comp));

        Action::Continue
    }

    fn end_loop(&mut self, _: &mut ir::Loop, _: &mut VisitorData) -> Action {
        self.pop();

        Action::Continue
    }

    fn end(&mut self, data: &mut VisitorData) {
        // Insert the asserts to the start of the component cmds
        let cmds = std::mem::take(&mut data.comp.cmds);
        let facts = std::mem::take(&mut self.facts);
        data.comp.cmds = facts.into_iter().chain(cmds).collect();
    }
}
