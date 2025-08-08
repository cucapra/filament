use super::{Bind, Foreign};
use crate::{
    AddCtx, CmpOp, Ctx, Event, Expr, ExprIdx, MutCtx, Param, Prop, PropIdx,
    Time, TimeIdx, TimeSub,
};

impl<C> Foreign<Expr, C>
where
    C: Ctx<Time>
        + Ctx<Expr>
        + Ctx<Prop>
        + Ctx<Param>
        + Ctx<Event>
        + AddCtx<Time>
        + AddCtx<Prop>
        + AddCtx<Expr>
        + MutCtx<Param>,
{
    pub fn transfer_with(
        &self,
        wctx: &mut C,
        rctx: &C,
        param_bind: &Bind<Foreign<Param, C>, ExprIdx>,
        event_bind: &Bind<Foreign<Event, C>, TimeIdx>,
    ) -> ExprIdx {
        let (e, owner) = self.take();
        match rctx.get(e) {
            Expr::Param(idx) => {
                let idx = Foreign::new(*idx, owner);
                *param_bind.get(&idx).unwrap()
            }
            Expr::Concrete(e) => wctx.add(Expr::Concrete(*e)),
            Expr::Bin { op, lhs, rhs } => {
                let lhs = Foreign::new(*lhs, owner);
                let rhs = Foreign::new(*rhs, owner);
                let lhs = lhs.transfer_with(wctx, rctx, param_bind, event_bind);
                let rhs = rhs.transfer_with(wctx, rctx, param_bind, event_bind);
                wctx.add(Expr::Bin { op: *op, lhs, rhs })
            }
            Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|arg| {
                        let arg = Foreign::new(*arg, owner);
                        arg.transfer_with(wctx, rctx, param_bind, event_bind)
                    })
                    .collect();
                wctx.add(Expr::Fn { op: *op, args })
            }
            Expr::If { cond, then, alt } => {
                let cond = Foreign::new(*cond, owner);
                let then = Foreign::new(*then, owner);
                let alt = Foreign::new(*alt, owner);
                let cond =
                    cond.transfer_with(wctx, rctx, param_bind, event_bind);
                let then =
                    then.transfer_with(wctx, rctx, param_bind, event_bind);
                let alt = alt.transfer_with(wctx, rctx, param_bind, event_bind);
                wctx.add(Expr::If { cond, then, alt })
            }
        }
    }
}

impl<C> Foreign<Prop, C>
where
    C: Ctx<Time>
        + Ctx<Expr>
        + Ctx<Prop>
        + Ctx<Param>
        + Ctx<Event>
        + AddCtx<Time>
        + AddCtx<Prop>
        + AddCtx<Expr>
        + MutCtx<Param>,
{
    pub fn transfer_with(
        &self,
        wctx: &mut C,
        rctx: &C,
        param_bind: &Bind<Foreign<Param, C>, ExprIdx>,
        event_bind: &Bind<Foreign<Event, C>, TimeIdx>,
    ) -> PropIdx {
        let (e, owner) = self.take();
        match rctx.get(e) {
            prop @ (Prop::True | Prop::False) => wctx.add(prop.clone()),
            Prop::Cmp(CmpOp { op, lhs, rhs }) => {
                let lhs = Foreign::new(*lhs, owner);
                let rhs = Foreign::new(*rhs, owner);
                let lhs = lhs.transfer_with(wctx, rctx, param_bind, event_bind);
                let rhs = rhs.transfer_with(wctx, rctx, param_bind, event_bind);
                wctx.add(Prop::Cmp(CmpOp { op: *op, lhs, rhs }))
            }
            Prop::TimeCmp(CmpOp { op, lhs, rhs }) => {
                let lhs = Foreign::new(*lhs, owner);
                let rhs = Foreign::new(*rhs, owner);
                let lhs = lhs.transfer_with(wctx, rctx, param_bind, event_bind);
                let rhs = rhs.transfer_with(wctx, rctx, param_bind, event_bind);
                wctx.add(Prop::TimeCmp(CmpOp { op: *op, lhs, rhs }))
            }
            Prop::TimeSubCmp(CmpOp { op, lhs, rhs }) => {
                let lhs = match lhs {
                    TimeSub::Unit(lhs) => {
                        let lhs = Foreign::new(*lhs, owner);
                        let lhs = lhs
                            .transfer_with(wctx, rctx, param_bind, event_bind);
                        TimeSub::Unit(lhs)
                    }
                    TimeSub::Sym { l, r } => {
                        let l = Foreign::new(*l, owner);
                        let r = Foreign::new(*r, owner);
                        let l =
                            l.transfer_with(wctx, rctx, param_bind, event_bind);
                        let r =
                            r.transfer_with(wctx, rctx, param_bind, event_bind);
                        TimeSub::Sym { l, r }
                    }
                };

                let rhs = match rhs {
                    TimeSub::Unit(rhs) => {
                        let rhs = Foreign::new(*rhs, owner);
                        let rhs = rhs
                            .transfer_with(wctx, rctx, param_bind, event_bind);
                        TimeSub::Unit(rhs)
                    }
                    TimeSub::Sym { l, r } => {
                        let l = Foreign::new(*l, owner);
                        let r = Foreign::new(*r, owner);
                        let l =
                            l.transfer_with(wctx, rctx, param_bind, event_bind);
                        let r =
                            r.transfer_with(wctx, rctx, param_bind, event_bind);
                        TimeSub::Sym { l, r }
                    }
                };

                wctx.add(Prop::TimeSubCmp(CmpOp { op: *op, lhs, rhs }))
            }
            Prop::Not(idx) => {
                let idx = Foreign::new(*idx, owner);
                let idx = idx.transfer_with(wctx, rctx, param_bind, event_bind);
                idx.not(wctx)
            }
            Prop::And(lhs, rhs) => {
                let lhs = Foreign::new(*lhs, owner);
                let rhs = Foreign::new(*rhs, owner);
                let lhs = lhs.transfer_with(wctx, rctx, param_bind, event_bind);
                let rhs = rhs.transfer_with(wctx, rctx, param_bind, event_bind);
                lhs.and(rhs, wctx)
            }
            Prop::Or(lhs, rhs) => {
                let lhs = Foreign::new(*lhs, owner);
                let rhs = Foreign::new(*rhs, owner);
                let lhs = lhs.transfer_with(wctx, rctx, param_bind, event_bind);
                let rhs = rhs.transfer_with(wctx, rctx, param_bind, event_bind);
                lhs.or(rhs, wctx)
            }
            Prop::Implies(prec, succ) => {
                let prec = Foreign::new(*prec, owner);
                let succ = Foreign::new(*succ, owner);
                let prec =
                    prec.transfer_with(wctx, rctx, param_bind, event_bind);
                let succ =
                    succ.transfer_with(wctx, rctx, param_bind, event_bind);
                prec.implies(succ, wctx)
            }
        }
    }
}

impl<C> Foreign<Time, C>
where
    C: Ctx<Time>
        + Ctx<Expr>
        + Ctx<Prop>
        + Ctx<Param>
        + Ctx<Event>
        + AddCtx<Prop>
        + AddCtx<Expr>
        + AddCtx<Time>
        + MutCtx<Param>,
{
    pub fn transfer_with(
        &self,
        wctx: &mut C,
        rctx: &C,
        param_bind: &Bind<Foreign<Param, C>, ExprIdx>,
        event_bind: &Bind<Foreign<Event, C>, TimeIdx>,
    ) -> TimeIdx {
        let (e, owner) = self.take();
        let Time { event, offset } = rctx.get(e);
        let event = Foreign::new(*event, owner);
        let offset = Foreign::new(*offset, owner);
        let time = *event_bind.get(&event).unwrap();
        let additional =
            offset.transfer_with(wctx, rctx, param_bind, event_bind);

        // Convert ['G + x] where 'G = 'H + y to ['H + x + y]

        let Time { event, offset } = wctx.get(time).clone();
        let offset = offset.add(additional, wctx);

        wctx.add(Time { event, offset })
    }
}
