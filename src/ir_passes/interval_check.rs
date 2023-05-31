use crate::ir::{self, Ctx};
use crate::ir_visitor::{Action, Visitor};

#[derive(Default)]
/// Filament's core interval checking algorithm. At a high-level it ensures that:
/// 1. All delays are well-formed
/// 2. Ports are connected for as long as expected
///
/// In order to ensure that delays are well-formed, we need to ensure that:
/// * Invocations provide events that trigger less often that expected by the
///   delay of the invoked component.
/// * The availability of bundle signals is less than the delay
/// * Shared instances are live for shorter duration than the delay
///
/// Like [super::TypeCheck], this pass simply generates all the assertions that
/// enforce the above constraints.
/// It is the job of a latter pass to ensure that the assertions are discharged.
pub struct IntervalCheck;

impl Visitor for IntervalCheck {
    /// For each event binding, we add the constraint that the events uses as arguments
    /// are triggered less often than the delay of the invoked component.
    fn event_binding(
        &mut self,
        eb: &mut ir::EventBind,
        comp: &mut ir::Component,
    ) -> Action {
        let ir::EventBind { event, arg } = &eb;
        let inv_ev = &comp[*event];
        let this_ev = &comp[comp[*arg].event];
        // Ensure that this event's delay is greater than invoked component's event's delay.
        let prop = comp.add(ir::Prop::TimeSubCmp(ir::CmpOp::gt(
            this_ev.delay.clone(),
            inv_ev.delay.clone(),
        )));
        let fact = ir::Command::from(comp.assert(prop));
        Action::AddBefore(vec![fact])
    }
}
