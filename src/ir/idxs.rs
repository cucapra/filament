use super::{
    Command, Component, Connect, Ctx, Event, Expr, If, Instance, Invoke, Loop,
    Param, Port, Prop, Time,
};
use crate::define_idx;

define_idx!(ParamIdx, Param, "pr");
define_idx!(EventIdx, Event, "ev");
define_idx!(TimeIdx, Time, "t");

define_idx!(ExprIdx, Expr, "e");
impl ExprIdx {
    /// Attempts to convert this expression into a concrete value.
    pub fn as_concrete(&self, ctx: &impl Ctx<Expr>) -> Option<u64> {
        if let Expr::Concrete(c) = ctx.get(*self) {
            Some(*c)
        } else {
            None
        }
    }

    /// Returns true if this expression is a constant.
    /// Note that this process *does not* automatically reduce the expression.
    /// For example, `1 + 1` is not going to be reduced to `2`.
    pub fn is_const(&self, ctx: &impl Ctx<Expr>, n: u64) -> bool {
        self.as_concrete(ctx).map(|c| c == n).unwrap_or(false)
    }

    /// Returns true of the expression is equal to the given parameter.
    pub fn is_param(&self, ctx: &impl Ctx<Expr>, param: ParamIdx) -> bool {
        if let Expr::Param(p) = ctx.get(*self) {
            *p == param
        } else {
            false
        }
    }
}

// Reference to a defined port-like value.
define_idx!(PortIdx, Port, "p");

impl PortIdx {
    /// Return true if this port is definitely not a bundle.
    /// This is the case if we can statically prove that the port has a length of 1.
    pub fn is_not_bundle<C>(&self, ctx: &C) -> bool
    where
        C: Ctx<Port> + Ctx<Expr>,
    {
        let port = ctx.get(*self);
        port.live.len.is_const(ctx, 1)
    }
}

define_idx!(PropIdx, Prop, "prop");

define_idx!(CmdIdx, Command, "cmd");
define_idx!(InstIdx, Instance, "inst");
define_idx!(InvIdx, Invoke, "inv");
define_idx!(ConIdx, Connect, "con");
define_idx!(IfIdx, If, "if");
define_idx!(LoopIdx, Loop, "loop");

define_idx!(CompIdx, Component, "comp");
