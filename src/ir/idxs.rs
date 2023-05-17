use super::{
    Command, Component, Connect, Ctx, Event, Expr, If, Instance, Invoke, Loop,
    Param, Port, Range, Time,
};
use crate::idx;

pub type ParamIdx = idx!(Param);
pub type EventIdx = idx!(Event);
pub type TimeIdx = idx!(Time);
pub type RangeIdx = idx!(Range);

pub type ExprIdx = idx!(Expr);
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

/// Reference to a defined port-like value.
pub type PortIdx = idx!(Port);
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

pub type CmdIdx = idx!(Command);
pub type InstIdx = idx!(Instance);
pub type InvIdx = idx!(Invoke);
pub type ConIdx = idx!(Connect);
pub type IfIdx = idx!(If);
pub type LoopIdx = idx!(Loop);

pub type CompIdx = idx!(Component);
