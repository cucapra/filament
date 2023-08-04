use crate::{
    ir,
    ir_visitor::{Action, Visitor},
};

#[derive(Default)]
/// Rewrite the control program so that uses of ports and invocations
/// dominated by their definitions.
///
/// Domination is achieved by hoisting all instances to the top of the scope
/// followed by invocation and finally connections and other control operations.
pub struct BuildDomination {
    /// Instances in the current stack of scopes.
    insts: Vec<Vec<ir::Command>>,
    /// Invocations in the current stack of scopes.
    invs: Vec<Vec<ir::Command>>,
}

impl BuildDomination {
    /// Start a new scope.
    fn start_scope(&mut self) {
        self.insts.push(Vec::new());
        self.invs.push(Vec::new());
    }

    /// End the current scope and return the instances and invocations
    /// in the scope.
    fn end_scope(&mut self) -> (Vec<ir::Command>, Vec<ir::Command>) {
        let Some(insts) = self.insts.pop() else {
            unreachable!("insts stack is empty")
        };
        let Some(invs) = self.invs.pop() else {
            unreachable!("invs stack is empty")
        };
        (insts, invs)
    }

    fn add_inv(&mut self, inv: ir::InvIdx) {
        self.invs.last_mut().unwrap().push(inv.into());
    }

    fn add_inst(&mut self, inst: ir::InstIdx) {
        self.insts.last_mut().unwrap().push(inst.into());
    }
}

impl Visitor for BuildDomination {
    fn name() -> &'static str {
        "build-domination"
    }

    fn invoke(&mut self, inv: ir::InvIdx, _: &mut ir::Component) -> Action {
        self.add_inv(inv);
        // Remove the invocation
        Action::Change(vec![])
    }

    fn instance(
        &mut self,
        inst: ir::InstIdx,
        _comp: &mut ir::Component,
    ) -> Action {
        self.add_inst(inst);
        // Remove the instance
        Action::Change(vec![])
    }

    fn start_cmds(&mut self, _: &mut Vec<ir::Command>, _: &mut ir::Component) {
        self.start_scope();
    }

    fn end_cmds(&mut self, cmds: &mut Vec<ir::Command>, _: &mut ir::Component) {
        let (inst, invs) = self.end_scope();
        // Insert instances and then invocations to the start of the scope.
        *cmds = inst
            .into_iter()
            .chain(invs.into_iter())
            .chain(cmds.drain(..))
            .collect();
    }

    fn end(&mut self, _: &mut ir::Component) {
        assert!(self.insts.is_empty());
        assert!(self.invs.is_empty());
    }
}
