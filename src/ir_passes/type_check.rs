use crate::{
    ir::{self, Ctx},
    ir_visitor::{Action, Visitor},
    utils::GPosIdx,
};

#[derive(Default)]
/// Implements the type checking algorithm for Filament.
/// It does things like:
/// * Port accesses are in-bounds
/// * Connections are between ports of same size
/// * Connected ports have the same bitwidths
pub struct TypeCheck;

impl TypeCheck {
    /// Generate constraints to ensure range accesses are within range and well-formed
    fn port_access(
        &mut self,
        access: &ir::Access,
        loc: GPosIdx,
        comp: &mut ir::Component,
    ) -> [ir::Fact; 3] {
        let &ir::Access { port, start, end } = access;
        let &ir::Port {
            live: ir::Liveness { len, .. },
            info,
            ..
        } = comp.get(port);

        let &ir::Info::Port {
            bind_loc, ..
        } = comp.get(info) else {
            unreachable!("Expected port info")
        };

        let wf = comp.add(
            ir::Reason::misc(
                "end of port access must greater than the start",
                loc,
            )
            .into(),
        );

        let wf_prop = end.gt(start, comp);
        let within_bounds =
            comp.add(ir::Reason::in_bounds_access(bind_loc, loc, len).into());
        let start = start.lt(len, comp);
        let end = end.lte(len, comp);
        [
            comp.assert(wf_prop, wf),
            comp.assert(start, within_bounds),
            comp.assert(end, within_bounds),
        ]
    }
}

impl Visitor for TypeCheck {
    fn connect(
        &mut self,
        c: &mut ir::Connect,
        comp: &mut ir::Component,
    ) -> Action {
        let ir::Connect { src, dst, info } = &c;
        let &ir::Info::Connect { dst_loc, src_loc } = comp.get(*info) else {
            unreachable!("Expected connect info")
        };
        let mut cons = vec![];

        // Range accesses are well-formed
        cons.extend(self.port_access(src, src_loc, comp));
        cons.extend(self.port_access(dst, dst_loc, comp));

        // Ensure that the bitwidths of the ports are the same
        let src_w = comp.get(src.port).width;
        let dst_w = comp.get(dst.port).width;
        let reason = comp.add(
            ir::Reason::misc(
                "connected ports must have the same bitwidth",
                GPosIdx::UNKNOWN,
            )
            .into(),
        );
        let prop = src_w.equal(dst_w, comp);
        cons.push(comp.assert(prop, reason));

        // Ensure that the sizes are the same
        let src_size = src.end.sub(src.start, comp);
        let dst_size = dst.end.sub(dst.start, comp);
        let reason = comp.add(
            ir::Reason::bundle_len_match(dst_loc, src_loc, dst_size, src_size)
                .into(),
        );
        let prop = src_size.equal(dst_size, comp);
        cons.push(comp.assert(prop, reason));

        Action::AddBefore(cons.into_iter().map(|f| f.into()).collect())
    }
}
