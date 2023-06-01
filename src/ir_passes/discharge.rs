use crate::ir_visitor::{Action, Visitor};
use crate::{ast, ir};
use easy_smt as smt;

/// Pass to discharge top-level `assert` statements in the IR and turn them into
/// `assume` if they are true. Any assertions within the body are left as-is.
/// Run [super::HoistFacts] before this pass to ensure that all facts are
/// top-level.
pub struct Discharge {
    sol: smt::Context,
    /// Are we in a scoped context?
    scoped: bool,
    // Defined names
    param_map: ir::DenseIndexInfo<ir::Param, smt::SExpr>,
    ev_map: ir::DenseIndexInfo<ir::Event, smt::SExpr>,
    // Composite expressions
    expr_map: ir::DenseIndexInfo<ir::Expr, smt::SExpr>,
    time_map: ir::DenseIndexInfo<ir::Time, smt::SExpr>,
    // Propositions
    prop_map: ir::DenseIndexInfo<ir::Prop, smt::SExpr>,
}

impl Default for Discharge {
    fn default() -> Self {
        let sol = smt::ContextBuilder::new()
            .replay_file(Some(std::fs::File::create("model.smt").unwrap()))
            .solver("z3", ["-smt2", "-in"])
            .build()
            .unwrap();
        Self {
            sol,
            scoped: false,
            param_map: Default::default(),
            prop_map: Default::default(),
            time_map: Default::default(),
            ev_map: Default::default(),
            expr_map: Default::default(),
        }
    }
}

impl Discharge {
    fn fmt_param(param: ir::ParamIdx) -> String {
        format!("param{}", param.get())
    }

    fn fmt_event(event: ir::EventIdx) -> String {
        format!("event{}", event.get())
    }

    fn fmt_expr(expr: ir::ExprIdx) -> String {
        format!("e{}", expr.get())
    }

    fn fmt_prop(prop: ir::PropIdx) -> String {
        format!("prop{}", prop.get())
    }

    fn fmt_time(time: ir::TimeIdx) -> String {
        format!("t{}", time.get())
    }

    /// Check whether the proposition is valid
    fn check_valid(&mut self, prop: ir::PropIdx) -> Option<bool> {
        self.sol.push().unwrap();
        let prop = self.prop_map[prop];
        self.sol.assert(self.sol.not(prop)).unwrap();
        let res = self.sol.check().unwrap();
        let out = match res {
            smt::Response::Sat => Some(false),
            smt::Response::Unsat => Some(true),
            smt::Response::Unknown => None,
        };
        self.sol.pop().unwrap();
        out
    }

    /// Generate constraint: `l <=> r`
    fn iff(&mut self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        self.sol.and(self.sol.imp(l, r), self.sol.imp(r, l))
    }

    fn expr_to_sexp(&mut self, expr: &ir::Expr) -> smt::SExpr {
        let sol = &mut self.sol;
        match expr {
            ir::Expr::Param(p) => self.param_map[*p],
            ir::Expr::Concrete(n) => sol.numeral(*n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let l = self.expr_map[*lhs];
                let r = self.expr_map[*rhs];
                match op {
                    ast::Op::Add => sol.plus(l, r),
                    ast::Op::Sub => sol.sub(l, r),
                    ast::Op::Mul => sol.times(l, r),
                    ast::Op::Div => sol.div(l, r),
                    ast::Op::Mod => sol.modulo(l, r),
                }
            }
            ir::Expr::Fn { .. } => todo!(),
        }
    }

    fn cmp_to_sexp<F, T>(
        &mut self,
        cmp: &ir::CmpOp<T>,
        mut transform: F,
    ) -> smt::SExpr
    where
        F: FnMut(&T, &Self) -> smt::SExpr,
    {
        let ir::CmpOp { op, lhs, rhs } = cmp;
        let l = transform(lhs, self);
        let r = transform(rhs, self);
        match op {
            ir::Cmp::Gt => self.sol.gt(l, r),
            ir::Cmp::Gte => self.sol.gte(l, r),
            ir::Cmp::Eq => self.sol.eq(l, r),
        }
    }

    /// Convert a proposition to an SMT expression.
    /// REQUIRES: Sub-terms mentioned in the proposition have already been defined.
    fn prop_to_sexp(&mut self, prop: &ir::Prop) -> smt::SExpr {
        let sol = &mut self.sol;
        match prop {
            ir::Prop::True => sol.true_(),
            ir::Prop::False => sol.false_(),
            ir::Prop::Cmp(c) => self.cmp_to_sexp(c, |e, ctx| ctx.expr_map[*e]),
            ir::Prop::TimeCmp(c) => {
                self.cmp_to_sexp(c, |t, ctx| ctx.time_map[*t])
            }
            ir::Prop::TimeSubCmp(c) => {
                self.cmp_to_sexp(c, |ts, ctx| match ts {
                    ir::TimeSub::Unit(e) => ctx.expr_map[*e],
                    ir::TimeSub::Sym { l, r } => {
                        let l = ctx.time_map[*l];
                        let r = ctx.time_map[*r];
                        ctx.sol.sub(l, r)
                    }
                })
            }
            ir::Prop::Not(p) => sol.not(self.prop_map[*p]),
            ir::Prop::And(l, r) => {
                let l = self.prop_map[*l];
                let r = self.prop_map[*r];
                sol.and(l, r)
            }
            ir::Prop::Or(l, r) => {
                let l = self.prop_map[*l];
                let r = self.prop_map[*r];
                sol.or(l, r)
            }
            ir::Prop::Implies(l, r) => {
                let l = self.prop_map[*l];
                let r = self.prop_map[*r];
                sol.imp(l, r)
            }
        }
    }
}

impl Visitor for Discharge {
    fn start(&mut self, comp: &mut ir::Component) -> Action {
        // Declare all parameters
        let int = self.sol.int_sort();
        for (idx, _) in comp.params.iter() {
            let sexp = self.sol.declare(Self::fmt_param(idx), int).unwrap();
            self.param_map.push(idx, sexp);
        }

        // Declare all events
        for (idx, _) in comp.events.iter() {
            let sexp = self.sol.declare(Self::fmt_event(idx), int).unwrap();
            self.ev_map.push(idx, sexp);
        }

        // Declare all expressions
        for (idx, expr) in comp.exprs.iter() {
            let sexp = self.sol.declare(Self::fmt_expr(idx), int).unwrap();
            let assign = self.expr_to_sexp(expr);
            let is_eq = self.sol.eq(sexp, assign);
            self.sol.assert(is_eq).unwrap();
            self.expr_map.push(idx, sexp);
        }

        // Declare all time expressions
        for (idx, ir::Time { event, offset }) in comp.times.iter() {
            let sexp = self.sol.declare(Self::fmt_time(idx), int).unwrap();
            let ev = self.ev_map[*event];
            let off = self.expr_map[*offset];
            let assign = self.sol.plus(ev, off);
            let is_eq = self.sol.eq(sexp, assign);
            self.sol.assert(is_eq).unwrap();
            self.time_map.push(idx, sexp);
        }

        // Declare all propositions
        let bs = self.sol.bool_sort();
        for (idx, prop) in comp.props.iter() {
            let sexp = self.sol.declare(Discharge::fmt_prop(idx), bs).unwrap();
            // Define assertion equating the proposition to its assignment
            let assign = self.prop_to_sexp(prop);
            let is_eq = self.iff(sexp, assign);
            self.sol.assert(is_eq).unwrap();
            self.prop_map.push(idx, sexp);
        }
        // Pass does not need to traverse the control program.
        Action::Continue
    }

    fn fact(&mut self, f: &mut ir::Fact, comp: &mut ir::Component) -> Action {
        if self.scoped {
            panic!("scoped facts not supported. Run `hoist-facts` before this pass");
        }

        if f.is_assume() {
            panic!(
                "assumptions should have been eliminated by `hoist-facts` pass"
            )
        }

        match self.check_valid(f.prop) {
            Some(true) => Action::Change(vec![comp.assume(f.prop).into()]),
            Some(false) => {
                log::warn!("fact `{}` is not valid", f.prop);
                Action::Continue
            }
            None => {
                panic!("Unknown returned")
            }
        }
    }

    fn do_if(&mut self, i: &mut ir::If, comp: &mut ir::Component) -> Action {
        let orig = self.scoped;
        self.scoped = true;
        let out = self
            .visit_cmds(&mut i.then, comp)
            .and_then(|| self.visit_cmds(&mut i.alt, comp));
        self.scoped = orig;
        out
    }

    fn do_loop(
        &mut self,
        l: &mut ir::Loop,
        comp: &mut ir::Component,
    ) -> Action {
        let orig = self.scoped;
        self.scoped = true;
        let out = self
            .start_loop(l, comp)
            .and_then(|| self.visit_cmds(&mut l.body, comp));
        self.scoped = orig;
        out
    }
}
