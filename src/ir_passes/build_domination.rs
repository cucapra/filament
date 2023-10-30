use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, Ctx};
use itertools::Itertools;
use topological_sort::TopologicalSort;

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
    /// Let bound parameters in the current stack of scopes.
    plets: Vec<Vec<ir::Command>>,
}

impl BuildDomination {
    /// Start a new scope.
    fn start_scope(&mut self) {
        self.insts.push(Vec::new());
        self.invs.push(Vec::new());
        self.plets.push(Vec::new());
    }

    /// Topologically sort the instances in the current scope.
    /// Dependency between instances is created when one uses a parameter
    /// defined by another.
    fn sort_insts(
        insts: Vec<ir::Command>,
        comp: &ir::Component,
    ) -> Vec<ir::Command> {
        let insts = insts
            .into_iter()
            .map(|i| {
                let ir::Command::Instance(inst) = i else {
                    unreachable!("expected instance command")
                };
                inst
            })
            .collect_vec();
        let mut topo =
            TopologicalSort::<ir::InstIdx>::from_iter(insts.iter().cloned());

        for inst in insts {
            let ir::Instance { args, .. } = comp.get(inst);
            for arg in args.iter() {
                let ir::Expr::Param(p_idx) = comp.get(*arg) else {
                    continue;
                };
                let ir::ParamOwner::Instance { inst: parent, .. } =
                    comp.get(*p_idx).owner
                else {
                    continue;
                };
                topo.add_dependency(parent, inst);
            }
        }

        topo.map(|i| i.into()).collect_vec()
    }

    /// End the current scope and return the instances and invocations
    /// in the scope.
    fn end_scope(
        &mut self,
        comp: &ir::Component,
    ) -> (Vec<ir::Command>, Vec<ir::Command>, Vec<ir::Command>) {
        let Some(insts) = self.insts.pop() else {
            unreachable!("insts stack is empty")
        };
        let Some(invs) = self.invs.pop() else {
            unreachable!("invs stack is empty")
        };
        let Some(plets) = self.plets.pop() else {
            unreachable!("plets stack is empty")
        };
        (Self::sort_insts(insts, comp), invs, plets)
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

    fn invoke(&mut self, inv: ir::InvIdx, _: &mut VisitorData) -> Action {
        self.add_inv(inv);
        // Remove the invocation
        Action::Change(vec![])
    }

    fn instance(&mut self, inst: ir::InstIdx, _: &mut VisitorData) -> Action {
        self.add_inst(inst);
        // Remove the instance
        Action::Change(vec![])
    }

    fn let_(&mut self, let_: &mut ir::Let, _: &mut VisitorData) -> Action {
        self.plets.last_mut().unwrap().push(let_.clone().into());
        // Remove the let
        Action::Change(vec![])
    }

    fn start_cmds(&mut self, _: &mut Vec<ir::Command>, _: &mut VisitorData) {
        self.start_scope();
    }
    fn end_cmds(&mut self, cmds: &mut Vec<ir::Command>, d: &mut VisitorData) {
        let (inst, invs, plets) = self.end_scope(&d.comp);
        // Insert instances and then invocations to the start of the scope.
        *cmds = plets
            .into_iter()
            .chain(inst)
            .chain(invs)
            .chain(cmds.drain(..))
            .collect();
    }

    fn end(&mut self, _: &mut VisitorData) {
        assert!(self.insts.is_empty());
        assert!(self.invs.is_empty());
    }
}
