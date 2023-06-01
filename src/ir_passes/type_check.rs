use crate::{
    ir::{self, Ctx},
    ir_visitor::{Action, Visitor},
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
        comp: &mut ir::Component,
    ) -> [ir::PropIdx; 3] {
        let &ir::Access { port, start, end } = access;
        let port = comp.get(port);
        let len = port.live.len;
        [end.gt(start, comp), start.lt(len, comp), end.lte(len, comp)]
    }
}

impl Visitor for TypeCheck {
    fn connect(
        &mut self,
        c: &mut ir::Connect,
        comp: &mut ir::Component,
    ) -> Action {
        let ir::Connect { src, dst, .. } = &c;
        let mut cons = vec![];

        // Range accesses are well-formed
        cons.extend(self.port_access(src, comp));
        cons.extend(self.port_access(dst, comp));

        // Ensure that the bitwidths of the ports are the same
        let src_w = comp.get(src.port).width;
        let dst_w = comp.get(dst.port).width;
        cons.push(src_w.equal(dst_w, comp));

        // Ensure that the sizes are the same
        let src_size = src.end.sub(src.start, comp);
        let dst_size = dst.end.sub(dst.start, comp);
        cons.push(src_size.equal(dst_size, comp));

        Action::AddBefore(
            cons.into_iter().map(|p| comp.assert(p).into()).collect(),
        )
    }
}
