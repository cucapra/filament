use fil_ir::{self as ir, AddCtx, Ctx};
pub struct Concretize<'comp> {
    comp: &'comp mut ir::Component,
    binding: ir::Bind<ir::ParamIdx, u64>,

    expr_map: ir::DenseIndexInfo<ir::Expr, ir::ExprIdx>,
    prop_map: ir::DenseIndexInfo<ir::Prop, ir::PropIdx>,
    time_map: ir::DenseIndexInfo<ir::Time, ir::TimeIdx>,

    inst_map: ir::DenseIndexInfo<ir::Instance, ir::InstIdx>,
    inv_map: ir::DenseIndexInfo<ir::Invoke, ir::InvIdx>,
    port_map: ir::DenseIndexInfo<ir::Port, ir::PortIdx>,
}

impl Concretize<'_> {}

impl Concretize<'_> {
    pub fn comp(&mut self) {}

    fn param(&mut self, param: ir::ParamIdx) -> ir::ExprIdx {
        if let Some(v) = self.binding.get(&param) {
            self.comp.add(ir::Expr::Concrete(*v))
        } else {
            self.comp.add(ir::Expr::Param(param))
        }
    }

    fn expr(&mut self, expr: ir::ExprIdx) -> ir::ExprIdx {
        if self.expr_map.contains(expr) {
            return *self.expr_map.get(expr);
        }

        let new_expr = match self.comp.get(expr).clone() {
            ir::Expr::Param(idx) => self.param(idx),
            ir::Expr::Concrete(_) => expr,
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(lhs);
                let rhs = self.expr(rhs);
                self.comp.add(ir::Expr::Bin { op, lhs, rhs })
            }
            ir::Expr::Fn { op, args } => {
                let args = args.into_iter().map(|arg| self.expr(arg)).collect();

                self.comp.add(ir::Expr::Fn { op, args })
            }
            ir::Expr::If { cond, then, alt } => {
                let cond = self.prop(cond);
                let then = self.expr(then);
                let alt = self.expr(alt);
                self.comp.add(ir::Expr::If { cond, then, alt })
            }
        };

        self.expr_map.push(expr, new_expr);
        new_expr
    }

    fn prop(&mut self, prop: ir::PropIdx) -> ir::PropIdx {
        if self.prop_map.contains(prop) {
            return *self.prop_map.get(prop);
        }

        let new_prop = match self.comp.get(prop).clone() {
            ir::Prop::True | ir::Prop::False => prop,
            ir::Prop::Cmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.expr(lhs);
                let rhs = self.expr(rhs);
                self.comp.add(ir::Prop::Cmp(ir::CmpOp { op, lhs, rhs }))
            }
            ir::Prop::TimeCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.time(lhs);
                let rhs = self.time(rhs);
                self.comp.add(ir::Prop::TimeCmp(ir::CmpOp { op, lhs, rhs }))
            }
            ir::Prop::TimeSubCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.timesub(lhs);
                let rhs = self.timesub(rhs);
                self.comp
                    .add(ir::Prop::TimeSubCmp(ir::CmpOp { op, lhs, rhs }))
            }
            ir::Prop::Not(idx) => {
                let idx = self.prop(idx);
                self.comp.add(ir::Prop::Not(idx))
            }
            ir::Prop::And(idx, idx1) => {
                let idx = self.prop(idx);
                let idx1 = self.prop(idx1);
                self.comp.add(ir::Prop::And(idx, idx1))
            }
            ir::Prop::Or(idx, idx1) => {
                let idx = self.prop(idx);
                let idx1 = self.prop(idx1);
                self.comp.add(ir::Prop::Or(idx, idx1))
            }
            ir::Prop::Implies(idx, idx1) => {
                let idx = self.prop(idx);
                let idx1 = self.prop(idx1);
                self.comp.add(ir::Prop::Implies(idx, idx1))
            }
        };

        self.prop_map.push(prop, new_prop);

        new_prop
    }

    fn time(&mut self, time: ir::TimeIdx) -> ir::TimeIdx {
        if self.time_map.contains(time) {
            return *self.time_map.get(time);
        }

        let ir::Time { event, offset } = self.comp.get(time).clone();

        let new_time = ir::Time {
            event,
            offset: self.expr(offset),
        };

        let new_time = self.comp.add(new_time);
        self.time_map.push(time, new_time);

        new_time
    }

    fn timesub(&mut self, timesub: ir::TimeSub) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(idx) => ir::TimeSub::Unit(self.expr(idx)),
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(l),
                r: self.time(r),
            },
        }
    }
}
