use itertools::Itertools;

use crate::ir::{self, Ctx};
use crate::ir_visitor::{Action, Visitor};
use crate::utils::{self, GPosIdx};

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

impl IntervalCheck {
    /// Constraints to ensure that the range is well-formed, i.e., the end of
    /// the range is strictly greater than the start.
    fn range_wf(
        &mut self,
        range: &ir::Range,
        loc: GPosIdx,
        comp: &mut ir::Component,
    ) -> ir::Command {
        let &ir::Range { start, end } = range;
        let prop = end.gt(start, comp);
        let reason = comp
            .add(ir::Reason::well_formed_interval(loc, (start, end)).into());
        comp.assert(prop, reason).into()
    }

    /// Check that event delays are greater than zero
    fn delay_wf(
        &mut self,
        event: ir::EventIdx,
        comp: &mut ir::Component,
    ) -> ir::Command {
        let zero = comp.num(0).into();
        let ir::Event { delay, info, .. } = &comp[event];
        let ir::Info::Event { delay_loc, .. } = comp[*info] else {
            unreachable!("expected event info")
        };
        let prop = delay.clone().gt(zero, comp);
        let reason = comp.add(
            ir::Reason::misc("delay must be greater than zero", delay_loc)
                .into(),
        );
        comp.assert(prop, reason).into()
    }
}

impl Visitor for IntervalCheck {
    fn start(&mut self, comp: &mut ir::Component) -> Action {
        // Ensure that delays are greater than zero
        let mut cmds = Vec::with_capacity(comp.ports.len() + comp.events.len());
        for idx in comp.events.idx_iter() {
            cmds.push(self.delay_wf(idx, comp));
        }

        // For each bundle, add an assertion to ensure that availability of the
        // bundle signal is less than the delay.
        // Extract the ranges first because we cannot borrow comp mutably before this.
        let ranges = comp
            .ports
            .iter()
            .map(|(_, p)| (p.live.clone(), p.info))
            .collect_vec();

        for (live, info) in ranges {
            let ir::Info::Port { live_loc, .. } = comp[info] else {
                unreachable!("expected port info")
            };
            let range = live.range;
            // Require that the range is well-formed
            cmds.push(self.range_wf(&range, live_loc, comp));

            // We only constraint the event mentioned in the start of the range.
            let st_ev = comp[range.start].event;
            let len = range.end.sub(range.start, comp);
            let ev = &comp[st_ev];
            let delay = ev.delay.clone();
            let ir::Info::Event { delay_loc, .. } = comp[ev.info] else {
                unreachable!("expected event info")
            };
            let param = comp.get(live.idx);
            let ir::Info::Param { bind_loc, .. } = comp[param.info] else {
                unreachable!("expected param info")
            };
            let zero = comp.num(0);
            let reason = comp.add(
                ir::Reason::bundle_delay(
                    delay_loc,
                    live_loc,
                    len.clone(),
                    bind_loc,
                    (zero, live.len),
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
        con: &mut ir::Connect,
        comp: &mut ir::Component,
    ) -> Action {
        let ir::Connect { src, dst, info } = con;
        let src_t = src.bundle_typ(comp);
        let dst_t = dst.bundle_typ(comp);
        // Assuming that the lengths are equal and parameters are in-range, check the constraint.
        let len_eq = src_t.len.equal(dst_t.len, comp);
        let contains = src_t
            .range
            .start
            .lte(dst_t.range.start, comp)
            .and(src_t.range.end.gte(dst_t.range.end, comp), comp);

        let ir::Info::Connect { dst_loc, src_loc } = comp.get(*info) else {
            unreachable!("Expected connect info")
        };
        let reason = comp.add(
            ir::Reason::liveness(*dst_loc, *src_loc, dst_t.range, src_t.range)
                .into(),
        );

        let prop = len_eq.implies(contains, comp);
        Action::AddBefore(vec![comp.assert(prop, reason).into()])
    }
}
