use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, AddCtx, Ctx, DisplayCtx, Foldable};
use fil_utils::GPosIdx;
use itertools::Itertools;

/// Generates default assumptions associated with
/// well-formed programs.
#[derive(Default)]
pub struct Assumptions;

impl Assumptions {
    fn transfer_prop(
        prop: ir::Foreign<ir::Prop, ir::Component>,
        data: &mut VisitorData,
        param_bind: &ir::Bind<
            ir::Foreign<ir::Param, ir::Component>,
            ir::ExprIdx,
        >,
        event_bind: &ir::Bind<
            ir::Foreign<ir::Event, ir::Component>,
            ir::TimeIdx,
        >,
    ) -> ir::PropIdx {
        if prop.owner() == data.idx {
            // The proposition is in the current component,
            // use ir::Subst instead of transfer
            let prop = prop.key();
            prop.fold_with(&mut data.comp, &mut |param| {
                param_bind.get(&ir::Foreign::new(param, data.idx)).copied()
            })
            .fold_with(&mut data.comp, &mut |event| {
                event_bind.get(&ir::Foreign::new(event, data.idx)).copied()
            })
        } else {
            // The proposition is in a different component
            // Thus, we can use data.mut_ctx.get directly to get the component
            prop.transfer_with(
                &mut data.comp,
                data.mut_ctx.get(prop.owner()),
                param_bind,
                event_bind,
            )
        }
    }

    /// Adds assumptions about the ports in the component
    fn port_assumptions(&mut self, data: &mut VisitorData) -> Vec<ir::Command> {
        let comp = &mut data.comp;
        let mut cmds = Vec::with_capacity(comp.ports().len() * 2);
        let ports = comp
            .ports()
            .iter()
            .flat_map(|(_, p)| {
                p.live.idxs.iter().copied().zip(p.live.lens.iter().copied())
            })
            .collect_vec();

        // Add assumptions for range of bundle-bound indices
        for (idx, len) in ports {
            let ir::Param { info, .. } = comp.get(idx);
            let bind_loc = if let Some(&ir::info::Param { bind_loc, .. }) =
                comp.get(*info).into()
            {
                bind_loc
            } else {
                GPosIdx::UNKNOWN
            };

            let idx = idx.expr(comp);
            let start = idx.gte(comp.num(0), comp);
            let end = idx.lt(len, comp);
            let in_range = start.and(end, comp);

            let reason = comp.add(
                ir::info::Reason::misc(
                    "bundle index is within range",
                    bind_loc,
                )
                .into(),
            );

            cmds.extend(comp.assume(in_range, reason))
        }
        cmds
    }

    /// Get the parameter binding for an instance
    fn param_binding(
        &self,
        inst: ir::InstIdx,
        data: &mut VisitorData,
    ) -> ir::Bind<ir::Foreign<ir::Param, ir::Component>, ir::ExprIdx> {
        let ir::Instance {
            comp: idx,
            args,
            params,
            ..
        } = data.comp.get(inst).clone();

        let comp = data.get(idx);
        let param_bind = comp
            .param_args()
            .iter()
            .copied()
            .chain(comp.exist_params())
            .map(|param| ir::Foreign::new(param, idx))
            .collect_vec();

        let param_args = args
            .iter()
            .copied()
            .chain(params.into_iter().map(|param| param.expr(&mut data.comp)))
            .collect_vec();

        ir::Bind::new(
            param_bind.into_iter().zip(param_args).collect::<Vec<_>>(),
        )
    }
}

impl Visitor for Assumptions {
    fn name() -> &'static str {
        "assumptions"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        // Intern all signature-level parameter assumptions
        let mut assumptions = Vec::new();

        for (prop, loc) in data
            .comp
            .get_event_asserts()
            .iter()
            .chain(data.comp.get_param_asserts())
            .copied()
            .collect_vec()
        {
            let info = data.comp.add(ir::Info::assert(ir::info::Reason::misc(
                "Signature assumption",
                loc,
            )));
            assumptions.extend(data.comp.assume(prop, info))
        }

        // Add assumptions about the ports in the component
        assumptions.extend(self.port_assumptions(data));

        Action::AddBefore(assumptions)
    }

    fn instance(
        &mut self,
        inst: fil_ir::InstIdx,
        data: &mut VisitorData,
    ) -> Action {
        let ir::Instance {
            comp: foreign_idx,
            info,
            ..
        } = data.comp.get(inst).clone();

        let info = data.comp.get(info);
        let comp_loc =
            if let Some(&ir::info::Instance { comp_loc, .. }) = info.into() {
                comp_loc
            } else {
                GPosIdx::UNKNOWN
            };
        let mut assumptions = Vec::new();

        log::trace!(
            "Transferring assumptions for instance {} from comp {} to {}",
            data.comp.display(inst),
            foreign_idx,
            data.idx
        );

        let param_bind = self.param_binding(inst, data);

        let param_asserts = data
            .get(foreign_idx)
            .get_param_asserts()
            .iter()
            .copied()
            .collect_vec();

        // Intern all instance-level param assertions in the foreign component
        // as assertions that need to be proved for the current component.
        for (prop, loc) in param_asserts {
            log::trace!(
                "Transferring parameter assumption {} from {} to {}",
                data.get(foreign_idx).display(prop),
                foreign_idx,
                data.idx
            );

            let prop = ir::Foreign::new(prop, foreign_idx);

            let new_prop = Assumptions::transfer_prop(
                prop,
                data,
                &param_bind,
                &ir::Bind::new(None),
            );

            let info = data.comp.add(ir::Info::assert(
                ir::info::Reason::param_cons(comp_loc, loc),
            ));

            assumptions.extend(data.comp.assert(new_prop, info))
        }

        let exist_assumes = data.get(foreign_idx).all_exist_assumes();

        // Existential assumptions in the foreign component become
        // assumptions in the current component.
        for (prop, loc) in exist_assumes {
            log::trace!(
                "Transferring existential assumption {} from {} to {}",
                data.get(foreign_idx).display(prop),
                foreign_idx,
                data.idx
            );

            let prop = ir::Foreign::new(prop, foreign_idx);

            let new_prop = Assumptions::transfer_prop(
                prop,
                data,
                &param_bind,
                &ir::Bind::new(None),
            );

            let info = data.comp.add(ir::Info::assert(
                ir::info::Reason::exist_cons(comp_loc, Some(loc)),
            ));

            assumptions.extend(data.comp.assume(new_prop, info))
        }

        Action::AddBefore(assumptions)
    }

    fn invoke(
        &mut self,
        inv: fil_ir::InvIdx,
        data: &mut VisitorData,
    ) -> Action {
        let ir::Invoke {
            inst, events, info, ..
        } = data.comp.get(inv).clone();

        let foreign_idx = inv.comp(&data.comp);

        let info: &fil_ir::Info = data.comp.get(info);
        let inst_loc =
            if let Some(&ir::info::Invoke { inst_loc, .. }) = info.into() {
                inst_loc
            } else {
                GPosIdx::UNKNOWN
            };
        let mut assumptions = Vec::new();

        log::trace!(
            "Transferring assumptions for invoke {} from comp {} to {}",
            data.comp.display(inv),
            foreign_idx,
            data.idx
        );

        let param_bind = self.param_binding(inst, data);

        let event_bind = ir::Bind::new(
            events
                .into_iter()
                .map(|ir::EventBind { arg, base, .. }| (base, arg)),
        );
        // We need to do this separately to avoid borrowing data.comp mutably while it is immutably borrowed.
        let event_asserts = data
            .get(foreign_idx)
            .get_event_asserts()
            .iter()
            .copied()
            .collect_vec();

        // Intern all instance-level param assertions in the foreign component
        // as assertions that need to be proved for the current component.
        for (prop, loc) in event_asserts {
            log::trace!(
                "Transferring event assertion {} from {} to {}",
                data.get(foreign_idx).display(prop),
                foreign_idx,
                data.idx
            );

            let prop = ir::Foreign::new(prop, foreign_idx);

            let new_prop = Assumptions::transfer_prop(
                prop,
                data,
                &param_bind,
                &event_bind,
            );

            let info = data.comp.add(ir::Info::assert(
                ir::info::Reason::event_cons(inst_loc, loc),
            ));

            assumptions.extend(data.comp.assert(new_prop, info))
        }

        Action::AddBefore(assumptions)
    }

    fn start_loop(
        &mut self,
        l: &mut fil_ir::Loop,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &mut data.comp;

        let ir::Loop {
            index, start, end, ..
        } = l;

        let start = *start;
        let end = *end;
        let index = *index;
        let ir::Param { info, .. } = comp.get(index);
        let bind_loc = if let Some(&ir::info::Param { bind_loc, .. }) =
            comp.get(*info).into()
        {
            bind_loc
        } else {
            GPosIdx::UNKNOWN
        };
        let index = index.expr(comp);

        // Add the assumption that the index is within bounds
        let idx_start = index.gte(start, comp);
        let idx_end = index.lt(end, comp);
        let in_range = idx_start.and(idx_end, comp);

        let info = comp.add(
            ir::info::Reason::misc("loop index is within range", bind_loc)
                .into(),
        );

        Action::AddBefore(vec![comp.assume(in_range, info).unwrap()])
    }
}
