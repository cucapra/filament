use itertools::Itertools;

use crate::ir::{self, Ctx};
use crate::ir_visitor::{Action, Visitor};
use crate::utils;

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
    fn start(&mut self, comp: &mut ir::Component) -> Action {
        // For each bundle, add an assertion to ensure that availability of the
        // bundle signal is less than the delay.

        // Extract the ranges first because we cannot borrow comp mutably before this.
        let ranges = comp
            .ports
            .iter()
            .map(|(_, p)| p.live.range.clone())
            .collect_vec();

        let mut cmds = Vec::with_capacity(comp.ports.len());
        for range in ranges {
            let st_ev = comp[range.start].event;
            let end_ev = comp[range.end].event;
            // NOTE(rachit): Not sure if this assertion is necessary because
            // only the event used in the start time should be bounded but
            // adding it here nonetheless.
            assert!(st_ev == end_ev, "Bundle ranges with different events");
            let len = range.end.sub(range.start, comp);
            let delay = &comp[st_ev].delay.clone();
            let reason = comp.add(
                ir::Reason::misc(
                    "bundle liveness must be less than delay",
                    utils::GPosIdx::UNKNOWN,
                )
                .into(),
            );
            let prop = comp
                .add(ir::Prop::TimeSubCmp(ir::CmpOp::gte(delay.clone(), len)));
            cmds.push(ir::Command::from(comp.assert(prop, reason)));
        }

        Action::AddBefore(cmds)
    }

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
        let prop = comp.add(ir::Prop::TimeSubCmp(ir::CmpOp::gte(
            this_ev.delay.clone(),
            inv_ev.delay.clone(),
        )));
        let reason = comp.add(ir::Reason::misc(
            "invoked component's delay must be less or equal to than this event's delay",
            utils::GPosIdx::UNKNOWN,
        ).into());
        let fact = ir::Command::from(comp.assert(prop, reason));
        Action::AddBefore(vec![fact])
    }

    fn connect(
        &mut self,
        _con: &mut ir::Connect,
        _comp: &mut ir::Component,
    ) -> Action {
        // TODO: Check that the signals are alive for as long as needed.
        Action::Continue
    }
}
