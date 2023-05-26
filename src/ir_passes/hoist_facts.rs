use linked_hash_set::LinkedHashSet;

use crate::{
    ir::{self, Ctx},
    ir_visitor::{Action, Visitor},
};

#[derive(Default)]
/// Hoist all [ir::Fact] from the control flow graph into the top level by
/// adding their path conditions.
pub struct HoistFacts {
    /// Stack of path conditions
    stack: Vec<usize>,
    /// The current path condition
    path_cond: Vec<ir::PropIdx>,
    /// Facts to be hoisted
    facts: LinkedHashSet<ir::Fact>,
}

impl HoistFacts {
    /// Push a new stack frame by tracking the number of added path conditions
    pub fn push(&mut self) {
        let len = self.path_cond.len();
        self.stack.push(len);
    }

    /// Pop the current stack frame
    pub fn pop(&mut self) {
        let props = self.stack.pop().unwrap();
        self.path_cond.truncate(props);
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

impl Visitor for HoistFacts {
    fn fact(
        &mut self,
        fact: &mut ir::Fact,
        comp: &mut ir::Component,
    ) -> Action {
        if fact.is_assume() {
            self.insert(fact.prop);
        } else {
            // Otherwise this is a checked assertion that needs to be hoisted.
            // Generate prop = path_cond -> fact.prop
            let cond = self.path_cond(comp).implies(fact.prop, comp);
            self.facts.insert(ir::Fact::assert(cond));
        }
        Action::Change(vec![])
    }

    fn do_if(&mut self, i: &mut ir::If, comp: &mut ir::Component) -> Action {
        log::warn!("Before then: {}", self.path_cond.len());
        self.push();
        self.insert(i.cond);
        let ac = self.visit_cmds(&mut i.then, comp);
        assert!(ac == Action::Continue);
        self.pop();
        log::warn!("After then: {}", self.path_cond.len());

        self.push();
        self.insert(i.cond.not(comp));
        let ac = self.visit_cmds(&mut i.alt, comp);
        assert!(ac == Action::Continue);
        self.pop();
        log::warn!("After else: {}", self.path_cond.len());

        Action::Continue
    }

    fn start_loop(
        &mut self,
        l: &mut ir::Loop,
        comp: &mut ir::Component,
    ) -> Action {
        self.push();
        let ir::Loop {
            index, start, end, ..
        } = l;
        let idx = index.expr(comp);
        let start = idx.gte(*start, comp);
        let end = idx.lt(*end, comp);
        self.insert(start.and(end, comp));

        Action::Continue
    }

    fn end_loop(&mut self, _l: &mut ir::Loop, _: &mut ir::Component) -> Action {
        self.pop();

        Action::Continue
    }

    fn end(&mut self, comp: &mut ir::Component) {
        // Insert the asserts to the start of the component cmds
        let cmds = std::mem::take(&mut comp.cmds);
        let facts = std::mem::take(&mut self.facts);
        comp.cmds = facts
            .into_iter()
            .map(ir::Command::Fact)
            .chain(cmds.into_iter())
            .collect();
    }
}
