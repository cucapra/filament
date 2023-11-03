use std::collections::HashMap;

use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, Ctx};
use ir::DisplayCtx;
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

    /// Topologically sort the plets and insts in the current scope.
    /// Dependencies are created between plets and instances when one uses a parameter defined from the other.
    /// defined by another.
    fn sort_insts_plets(
        insts: Vec<ir::Command>,
        plets: Vec<ir::Command>,
        comp: &ir::Component,
    ) -> Vec<ir::Command> {
        // Combine the instances and plets into a single vector so that we can give them each unique ids
        let cmds: Vec<_> = insts.into_iter().chain(plets).collect();

        // Map from parameter to the id of the command that defines it.
        let mut param_map: HashMap<ir::Idx<ir::Param>, usize> = HashMap::new();

        // First pass to register all paremeter owners.
        for (id, cmd) in cmds.iter().enumerate() {
            match cmd {
                ir::Command::Let(ir::Let { param, .. }) => {
                    log::trace!("let-bound param {}", comp.display(*param));
                    param_map.insert(*param, id);
                }
                ir::Command::Instance(inst) => {
                    let ir::Instance { params, .. } = comp.get(*inst);
                    for param in params {
                        log::trace!(
                            "param {} is owned by {}",
                            comp.display(*param),
                            comp.display(*inst)
                        );
                        param_map.insert(*param, id);
                    }
                }
                _ => unreachable!(
                    "Expected Only expected param-lets and instances."
                ),
            }
        }

        let mut topo = TopologicalSort::<usize>::new();

        // Second pass to add dependencies between parameter owners and users.
        for (id, cmd) in cmds.iter().enumerate() {
            match cmd {
                ir::Command::Let(ir::Let { expr, param }) => {
                    for idx in expr.relevant_vars(comp) {
                        log::trace!(
                            "param {} is used by {}",
                            comp.display(idx),
                            comp.display(*param)
                        );
                        if let Some(pid) = param_map.get(&idx) {
                            // Add a dependency from the let to the parameter owner, if it is defined in this scope
                            topo.add_dependency(*pid, id);
                        }
                    }
                }
                ir::Command::Instance(inst) => {
                    for idx in (*inst).relevant_vars(comp) {
                        log::trace!(
                            "param {} is used by {}",
                            comp.display(idx),
                            comp.display(*inst)
                        );
                        if let Some(pid) = param_map.get(&idx) {
                            // Add a dependency from the instance to the parameter owner, if it is defined in this scope
                            topo.add_dependency(*pid, id);
                        }
                    }
                }
                _ => unreachable!(
                    "Expected Only expected param-lets and instances."
                ),
            }
        }

        // Wrap in option here because we need to be able to pull the commands out of the vector
        // without changing the indices of the remaining commands.
        let mut cmds = cmds.into_iter().map(Some).collect_vec();

        let mut res = Vec::new();

        // Add the commands in topological dependency order
        for i in topo {
            let Some(cmd) = cmds[i].take() else {
                unreachable!("Topological sort returned the same index twice.")
            };
            res.push(cmd);
        }

        // Add the remaining commands that were not dependent on anything
        res.extend(cmds.into_iter().flatten());
        res
    }

    /// End the current scope and return the instances and invocations
    /// in the scope.
    fn end_scope(
        &mut self,
        comp: &ir::Component,
    ) -> (Vec<ir::Command>, Vec<ir::Command>) {
        let Some(insts) = self.insts.pop() else {
            unreachable!("insts stack is empty")
        };
        let Some(invs) = self.invs.pop() else {
            unreachable!("invs stack is empty")
        };
        let Some(plets) = self.plets.pop() else {
            unreachable!("plets stack is empty")
        };
        (Self::sort_insts_plets(insts, plets, comp), invs)
    }

    fn add_inv(&mut self, inv: ir::InvIdx) {
        self.invs.last_mut().unwrap().push(inv.into());
    }

    fn add_inst(&mut self, inst: ir::InstIdx) {
        self.insts.last_mut().unwrap().push(inst.into());
    }

    fn add_let(&mut self, let_: ir::Let) {
        self.plets.last_mut().unwrap().push(let_.into());
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
        self.add_let(let_.clone());
        // Remove the let
        Action::Change(vec![])
    }

    fn start_cmds(&mut self, _: &mut Vec<ir::Command>, _: &mut VisitorData) {
        self.start_scope();
    }
    fn end_cmds(&mut self, cmds: &mut Vec<ir::Command>, d: &mut VisitorData) {
        let (inst_plets, invs) = self.end_scope(&d.comp);
        // Insert instances and then invocations to the start of the scope.
        *cmds = inst_plets
            .into_iter()
            .chain(invs)
            .chain(cmds.drain(..))
            .collect();
    }

    fn end(&mut self, _: &mut VisitorData) {
        assert!(self.insts.is_empty());
        assert!(self.invs.is_empty());
    }
}
