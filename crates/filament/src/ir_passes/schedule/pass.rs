use super::{Retime, Solve};
use crate::ir_visitor::Visitor;
use fil_ir as ir;

/// Sets the proper FSM Attributes for every component
#[derive(Default)]
pub struct Schedule;

// impl Schedule {
//     fn comp(&self, idx: &mut ir::Component) -> &mut ir::Component {

//     }
// }

impl Visitor for Schedule {
    fn name() -> &'static str {
        "schedule"
    }

    fn do_pass(
        opts: &crate::cmdline::Opts,
        ctx: &mut fil_ir::Context,
    ) -> Result<(), u64> {
        Solve::do_pass(opts, ctx)?;
        Retime::do_pass(opts, ctx)
    }
}
