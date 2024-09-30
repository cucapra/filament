use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ast as ast;
use fil_ir as ir;
use topological_sort::TopologicalSort;

/// Sets the proper FSM Attributes for every component
#[derive(Default)]
pub struct Schedule {
    topo: TopologicalSort<ir::PortIdx>,
}

impl Visitor for Schedule {
    fn name() -> &'static str {
        "schedule"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        // Quit the pass if this attribute does not have the #[schedule] attribute
        if !data.comp.attrs.has(ast::BoolAttr::Schedule) {
            Action::Stop
        } else {
            Action::Continue
        }
    }

    fn connect(
        &mut self,
        con: &mut ir::Connect,
        data: &mut VisitorData,
    ) -> Action {
        let ir::Connect { src, dst, .. } = con;

        todo!()
    }
}
