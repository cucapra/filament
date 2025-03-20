use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, AddCtx, Ctx};
use fil_utils::GPosIdx;
use itertools::Itertools;

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
        // The path condition
        pc: ir::PropIdx,
        range: &ir::Range,
        loc: GPosIdx,
        comp: &mut ir::Component,
    ) -> Option<ir::Command> {
        let &ir::Range { start, end } = range;
        let prop = end.gt(start, comp);
        let reason = comp.add(
            ir::info::Reason::well_formed_interval(loc, (start, end)).into(),
        );
        let prop = pc.implies(prop, comp);
        comp.assert(prop, reason)
    }

    /// Check that event delays are greater than zero
    fn delay_wf(
        &mut self,
        // The path condition
        pc: ir::PropIdx,
        event: ir::EventIdx,
        comp: &mut ir::Component,
    ) -> Option<ir::Command> {
        let zero = comp.num(0).into();
        let ir::Event { delay, info, .. } = &comp[event];
        let &ir::info::Event { delay_loc, .. } = comp.get(*info).into();
        let prop = pc.implies(delay.clone().gt(zero, comp), comp);
        let reason = comp.add(
            ir::info::Reason::misc(
                "delay must be greater than zero",
                delay_loc,
            )
            .into(),
        );
        comp.assert(prop, reason)
    }

    /// Proposition that ensures that the given parameter is in range
    fn in_range(live: &ir::Liveness, comp: &mut ir::Component) -> ir::PropIdx {
        let &ir::Liveness { idxs, lens, .. } = &live;
        let mut prop = comp.add(ir::Prop::True);
        for (idx, len) in idxs.iter().zip_eq(lens) {
            let zero = comp.num(0);
            let idx = idx.expr(comp);
            let lo = idx.gte(zero, comp);
            let hi = idx.lt(*len, comp);
            prop = prop.and(lo.and(hi, comp), comp);
        }
        prop
    }

    /// For each event binding, we add the constraint that the events uses as arguments
    /// are triggered less often than the delay of the invoked component.
    fn event_binding(
        &mut self,
        eb: ir::EventBind,
        comp: &mut ir::Component,
    ) -> Option<ir::Command> {
        let ir::EventBind {
            delay: inv_delay,
            arg,
            info,
            ..
        } = &eb;
        let &ir::info::EventBind {
            ev_delay_loc,
            bind_loc,
        } = comp.get(*info).into();

        let this_ev = &comp[comp[*arg].event];
        let this_delay = this_ev.delay.clone();
        let &ir::info::Event {
            delay_loc: ev_del_loc,
            ..
        } = comp.get(this_ev.info).into();

        let reason = comp.add(
            ir::info::Reason::event_trig(
                ev_delay_loc,
                inv_delay.clone(),
                ev_del_loc,
                this_delay.clone(),
                bind_loc,
            )
            .into(),
        );

        // Ensure that this event's delay is greater than invoked component's event's delay.
        let prop = this_delay.gte(inv_delay.clone(), comp);
        comp.assert(prop, reason)
    }
}

impl Visitor for IntervalCheck {
    fn name() -> &'static str {
        "interval-check"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        let comp = &mut data.comp;

        // Assertions about the signature get to use the constraints on existential parameters.
        let init = comp.add(ir::Prop::True);
        let assumes = comp
            .all_exist_assumes()
            .into_iter()
            .fold(init, |a, (b, _)| a.and(b, comp));

        // Ensure that delays are greater than zero
        let mut cmds: Vec<ir::Command> =
            Vec::with_capacity(comp.ports().len() + comp.events().len());
        for idx in comp.events().idx_iter() {
            cmds.extend(self.delay_wf(assumes, idx, comp));
        }

        // For each bundle, add an assertion to ensure that availability of the
        // bundle signal is less than the delay.
        // Extract the ranges first because we cannot borrow comp mutably before this.
        let ranges = comp
            .ports()
            .iter()
            .filter_map(|(_, p)| {
                // Ignore ports on invokes
                if !matches!(p.owner, ir::PortOwner::Inv { .. }) {
                    Some((p.live.clone(), p.info))
                } else {
                    None
                }
            })
            .collect_vec();

        for (live, info) in ranges {
            let &ir::info::Port { live_loc, .. } = comp.get(info).into();
            let range = live.range;
            // Require that the range is well-formed
            cmds.extend(self.range_wf(assumes, &range, live_loc, comp));

            // We only constraint the event mentioned in the start of the range.
            let st_ev = comp[range.start].event;
            let len = range.end.sub(range.start, comp);
            let ev = &comp[st_ev];
            let delay = ev.delay.clone();
            let &ir::info::Event { delay_loc, .. } = comp.get(ev.info).into();

            let param_info = live
                .idxs
                .iter()
                .zip_eq(&live.lens)
                .map(|(param, len)| {
                    let param = comp.get(*param);
                    let &ir::info::Param { bind_loc, .. } =
                        comp.get(param.info).into();
                    let zero = comp.num(0);
                    (bind_loc, zero, *len)
                })
                .collect_vec();
            let reason = comp.add(
                ir::info::Reason::bundle_delay(
                    delay_loc,
                    live_loc,
                    len.clone(),
                    param_info,
                )
                .into(),
            );
            let prop = delay.clone().gte(len, comp);
            let imp = assumes.implies(prop, comp);
            cmds.extend(comp.assert(imp, reason));
        }
        Action::AddBefore(cmds)
    }

    fn invoke(
        &mut self,
        inv_idx: ir::InvIdx,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &mut data.comp;
        let inst_idx = inv_idx.inst(comp);
        let lives = comp.get(inst_idx).lives.clone();
        let events = &comp[inv_idx].events.clone();
        let inv_info = comp.get(comp.get(inv_idx).info).as_invoke().cloned();

        let mut cmds = Vec::default();
        // If the liveness is defined, then ensure that the active range of the
        // event is in range.
        if !lives.is_empty() {
            let info = comp.get(comp.get(inst_idx).info).as_instance().cloned();
            for (i, (ir::Range { start, end }, live)) in
                lives.iter().zip_eq(events).enumerate()
            {
                let ir::EventBind {
                    delay,
                    arg: use_start,
                    ..
                } = live;
                let start_after = use_start.gte(*start, comp);
                let use_end = use_start.add(delay, comp);
                let end_before = use_end.lte(*end, comp);

                // Location information
                let info = if let (
                    Some(ir::info::Instance { event_lives, .. }),
                    Some(ir::info::Invoke {
                        event_bind_locs, ..
                    }),
                ) = (&info, &inv_info)
                {
                    let live_loc = event_lives[i];
                    let borrow = (*start, *end);
                    let inv_range = (*use_start, use_end);
                    ir::Info::assert(ir::info::Reason::event_live(
                        live_loc,
                        borrow,
                        inv_range,
                        event_bind_locs[i],
                    ))
                } else {
                    ir::Info::empty()
                };

                let info = comp.add(info);
                let prop = start_after.and(end_before, comp);
                let cmd = comp.assert(prop, info);
                cmds.extend(cmd);
            }
        }

        // Clone here because we need to pass mutable ownership of the component
        for eb in events.clone() {
            if let Some(assert) = self.event_binding(eb, comp) {
                cmds.push(assert)
            }
        }
        Action::AddBefore(cmds)
    }

    fn instance(
        &mut self,
        inst_idx: ir::InstIdx,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &mut data.comp;
        let inst = &comp[inst_idx];
        let info = comp[inst.info].as_instance().cloned();
        if inst.lives.is_empty() {
            return Action::Continue;
        }

        let mut cmds = Vec::with_capacity(inst.lives.len());
        for (i, ir::Range { start, end }) in
            inst.lives.clone().into_iter().enumerate()
        {
            // The range of this liveness
            let len = end.sub(start, comp);
            let inst = &comp[comp[start].event];
            let delay = inst.delay.clone();
            let ev_info = comp[inst.info].as_event().cloned();
            let prop = delay.clone().gte(len.clone(), comp);

            let reason = if let (
                Some(ir::info::Instance { event_lives, .. }),
                Some(ir::info::Event { delay_loc, .. }),
            ) = (&info, ev_info)
            {
                ir::Info::assert(ir::info::Reason::event_live_delay(
                    event_lives[i],
                    len,
                    delay_loc,
                    delay,
                ))
            } else {
                ir::Info::empty()
            };
            let info = comp.add(reason);
            cmds.extend(comp.assert(prop, info));
        }

        Action::AddBefore(cmds)
    }

    fn connect(
        &mut self,
        con: &mut ir::Connect,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &mut data.comp;
        let ir::Connect { src, dst, info } = con;
        let src_t = src.bundle_typ(comp);
        let dst_t = dst.bundle_typ(comp);
        let in_range = Self::in_range(&dst_t, comp)
            .and(Self::in_range(&src_t, comp), comp);

        // Substitute the parameter used in source with that in dst
        let binding = ir::Bind::new(
            dst_t
                .idxs
                .iter()
                // We only substitute the indices that appear in both
                .zip(&src_t.idxs)
                .map(|(d, s)| (*d, s.expr(comp)))
                .collect_vec(),
        );

        let dst_range =
            ir::Subst::new(dst_t.range.clone(), &binding).apply(comp);

        // Assuming that lengths are equal
        let one = comp.num(1);
        let s_len = src_t.lens.iter().fold(one, |acc, l| acc.mul(*l, comp));
        let d_len = dst_t.lens.iter().fold(one, |acc, l| acc.mul(*l, comp));

        let pre_req = s_len.equal(d_len, comp).and(in_range, comp);

        let contains = src_t
            .range
            .start
            .lte(dst_range.start, comp)
            .and(src_t.range.end.gte(dst_range.end, comp), comp);

        let &ir::info::Connect { src_loc, .. } = comp.get(*info).into();
        let reason = comp.add(
            ir::info::Reason::liveness(src_loc, dst_t.range, src_t.range)
                .into(),
        );

        let prop = pre_req.implies(contains, comp);
        let f = comp.assert(prop, reason);
        if let Some(c) = f {
            Action::AddBefore(vec![c])
        } else {
            Action::Continue
        }
    }
}
