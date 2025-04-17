use fil_ir as ir;

use super::{retime::Retime, solve::Solve};

pub fn schedule(
    ctx: &ir::Context,
    comp: &mut ir::Component,
    delay_register: ir::CompIdx,
    replay_file: Option<&String>,
) {
    Solve::new(comp, replay_file).comp();
    Retime::new(ctx, comp, delay_register).comp();
}
