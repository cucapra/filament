use crate::ir_visitor::{Action, Visitor, VisitorData};
use fil_ir::{self as ir, AddCtx, Ctx};
use fil_utils::GPosIdx;
use itertools::Itertools;

#[derive(Default)]
/// Implements the type checking algorithm for Filament.
/// It does things like:
/// * Port accesses are in-bounds
/// * Connections are between ports of same size
/// * Connected ports have the same bitwidths
/// * Add constraints on existentially quantified parameters
pub struct TypeCheck;

impl TypeCheck {
    /// Generate constraints to ensure range accesses are within range and well-formed
    fn port_access(
        &mut self,
        access: &ir::Access,
        loc: GPosIdx,
        comp: &mut ir::Component,
    ) -> impl Iterator<Item = ir::Command> {
        let &ir::Access { port, start, end } = access;
        let &ir::Port {
            live: ir::Liveness { len, .. },
            info,
            ..
        } = comp.get(port);

        let &ir::info::Port { bind_loc, .. } = comp.get(info).into();

        let wf = comp.add(
            ir::info::Reason::misc(
                "end of port access must greater than the start",
                loc,
            )
            .into(),
        );

        let wf_prop = end.gt(start, comp);
        let within_bounds = comp
            .add(ir::info::Reason::in_bounds_access(bind_loc, loc, len).into());
        let start = start.lt(len, comp);
        let end = end.lte(len, comp);
        let in_range = start.and(end, comp);
        vec![
            comp.assert(wf_prop, wf),
            comp.assert(in_range, within_bounds),
        ]
        .into_iter()
        .flatten()
    }
}

impl Visitor for TypeCheck {
    fn name() -> &'static str {
        "type-check"
    }

    fn exists(&mut self, e: &mut ir::Exists, data: &mut VisitorData) -> Action {
        let ctx = &mut data.comp;
        // Ensure that the parameter is an existentially quantified parameter.
        // XXX(rachit): This should really be a check in the validate pass but
        // currently don't run that pass.
        let Some(assumes) = ctx.get_exist_assumes(e.param) else {
            return Action::Continue;
        };
        let param = ctx.get(e.param);
        let info = ctx.get(param.info).as_param();
        let reason: ir::Info = info
            .map(|p| ir::info::Reason::exist_cons(p.bind_loc, None).into())
            .unwrap_or_default();
        let info = ctx.add(reason);

        Action::AddBefore(
            assumes
                .iter()
                .flat_map(|p| data.comp.assert(*p, info))
                .collect_vec(),
        )
    }

    fn connect(
        &mut self,
        c: &mut ir::Connect,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &mut data.comp;
        let ir::Connect { src, dst, info } = &c;
        let &ir::info::Connect { dst_loc, src_loc } = comp.get(*info).into();
        let mut cons = vec![];

        // Range accesses are well-formed
        cons.extend(self.port_access(src, src_loc, comp));
        cons.extend(self.port_access(dst, dst_loc, comp));

        // Ensure that the bitwidths of the ports are the same
        let src_w = comp.get(src.port).width;
        let dst_w = comp.get(dst.port).width;
        let reason = comp.add(
            ir::info::Reason::bundle_width_match(
                dst_loc, src_loc, dst_w, src_w,
            )
            .into(),
        );
        let prop = src_w.equal(dst_w, comp);
        cons.extend(comp.assert(prop, reason));

        // Ensure that the sizes are the same
        let src_size = src.end.sub(src.start, comp);
        let dst_size = dst.end.sub(dst.start, comp);
        let reason = comp.add(
            ir::info::Reason::bundle_len_match(
                dst_loc, src_loc, dst_size, src_size,
            )
            .into(),
        );
        let prop = src_size.equal(dst_size, comp);
        cons.extend(comp.assert(prop, reason));

        Action::AddBefore(cons)
    }
}
