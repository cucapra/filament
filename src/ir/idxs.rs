use super::{
    Component, Ctx, Event, Expr, Info, Instance, Invoke, Param, Port, Prop,
    Time,
};
use crate::define_idx;

define_idx!(ParamIdx, Param, "pr");
impl ParamIdx {
    /// Return an expression that refers to this parameter.
    pub fn expr<C: Ctx<Expr>>(self, ctx: &mut C) -> ExprIdx {
        ctx.add(Expr::Param(self))
    }
}

define_idx!(EventIdx, Event, "ev");
define_idx!(TimeIdx, Time, "t");
define_idx!(ExprIdx, Expr, "e");

// Reference to a defined port-like value.
define_idx!(PortIdx, Port, "p");
impl PortIdx {
    /// Return true if this port is definitely not a bundle.
    /// This is the case if we can statically prove that the port has a length of 1.
    pub fn is_not_bundle(&self, ctx: &Component) -> bool {
        let port = ctx.get(*self);
        port.live.len.is_const(ctx, 1)
    }
}

define_idx!(PropIdx, Prop, "prop");
define_idx!(InfoIdx, Info, "info");

define_idx!(InstIdx, Instance, "inst");
define_idx!(InvIdx, Invoke, "inv");

define_idx!(CompIdx, Component, "comp");
