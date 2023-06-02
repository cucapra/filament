use crate::ir::Ctx;
use crate::ir_visitor::{Action, Visitor};
use crate::utils::GlobalPositionTable;
use crate::{ast, ir, utils};
use codespan_reporting::{diagnostic as cr, term};
use easy_smt as smt;
use itertools::Itertools;
use std::collections::HashMap;
use std::iter;
use term::termcolor::{ColorChoice, StandardStream};

type Assign = Vec<(ir::ParamIdx, String)>;

/// Pass to discharge top-level `assert` statements in the IR and turn them into
/// `assume` if they are true. Any assertions within the body are left as-is.
/// Run [super::HoistFacts] before this pass to ensure that all facts are
/// top-level.
pub struct Discharge {
    sol: smt::Context,
    /// Are we in a scoped context?
    scoped: bool,
    /// Defined functions
    func_map: HashMap<ast::UnFn, smt::SExpr>,
    // Defined names
    param_map: ir::DenseIndexInfo<ir::Param, smt::SExpr>,
    ev_map: ir::DenseIndexInfo<ir::Event, smt::SExpr>,
    // Composite expressions
    expr_map: ir::DenseIndexInfo<ir::Expr, smt::SExpr>,
    time_map: ir::DenseIndexInfo<ir::Time, smt::SExpr>,
    // Propositions
    prop_map: ir::DenseIndexInfo<ir::Prop, smt::SExpr>,
    // Propositions that have already been checked
    checked: HashMap<ir::PropIdx, Option<Assign>>,
    // Diagnostics to be reported
    diagnostics: Vec<cr::Diagnostic<usize>>,
}

impl Default for Discharge {
    fn default() -> Self {
        let sol = smt::ContextBuilder::new()
            .replay_file(Some(std::fs::File::create("model.smt").unwrap()))
            .solver("z3", ["-smt2", "-in"])
            .build()
            .unwrap();

        let mut out = Self {
            sol,
            scoped: false,
            func_map: Default::default(),
            param_map: Default::default(),
            prop_map: Default::default(),
            time_map: Default::default(),
            ev_map: Default::default(),
            expr_map: Default::default(),
            checked: Default::default(),
            diagnostics: Default::default(),
        };

        out.define_funcs();
        out
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

    /// Defines primitive functions used in the encoding like `pow` and `log`
    fn define_funcs(&mut self) {
        let int_sort = self.sol.int_sort();
        let pow2 = self
            .sol
            .declare_fun("pow2", vec![int_sort], int_sort)
            .unwrap();
        let log = self
            .sol
            .declare_fun("log2", vec![int_sort], int_sort)
            .unwrap();
        self.func_map = vec![(ast::UnFn::Pow2, pow2), (ast::UnFn::Log2, log)]
            .into_iter()
            .collect();
    }

    /// Get bindings for the provided parameters in a model.
    fn get_assignments(&mut self, relevant_vars: Vec<ir::ParamIdx>) -> Assign {
        let mut rev_map = HashMap::with_capacity(relevant_vars.len());
        // SExprs corresponding to the paramters
        let sexps = relevant_vars
            .iter()
            .unique()
            .map(|p| {
                let s = self.param_map[*p];
                rev_map.insert(s, *p);
                s
            })
            .collect_vec();

        self.sol
            .get_value(sexps)
            .unwrap()
            .into_iter()
            .map(|(p, v)| {
                let p = rev_map[&p];
                (p, self.sol.display(v).to_string())
            })
            .collect_vec()
    }

    /// Check whether the proposition is valid.
    /// Returns a set of assignments if the proposition is not valid.
    fn check_valid(
        &mut self,
        prop: ir::PropIdx,
        ctx: &ir::Component,
    ) -> &Option<Assign> {
        #[allow(clippy::map_entry)]
        if !self.checked.contains_key(&prop) {
            self.sol.push().unwrap();
            self.sol.assert(self.sol.not(self.prop_map[prop])).unwrap();
            let res = self.sol.check().unwrap();
            let out = match res {
                smt::Response::Sat => {
                    Some(self.get_assignments(ctx.prop_params(prop)))
                }
                smt::Response::Unsat => None,
                smt::Response::Unknown => panic!("Solver returned unknown"),
            };
            self.sol.pop().unwrap();
            self.checked.insert(prop, out);
        }
        &self.checked[&prop]
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
            ir::Expr::Fn { op, args } => {
                let args = args.iter().map(|e| self.expr_map[*e]);
                self.sol.list(
                    iter::once(self.func_map[op]).chain(args).collect_vec(),
                )
            }
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
            let assign = self.expr_to_sexp(expr);
            let sexp = self
                .sol
                .define_const(Self::fmt_expr(idx), int, assign)
                .unwrap();
            self.expr_map.push(idx, sexp);
        }

        // Declare all time expressions
        for (idx, ir::Time { event, offset }) in comp.times.iter() {
            let assign =
                self.sol.plus(self.ev_map[*event], self.expr_map[*offset]);
            let sexp = self
                .sol
                .define_const(Self::fmt_time(idx), int, assign)
                .unwrap();
            self.time_map.push(idx, sexp);
        }

        // Declare all propositions
        let bs = self.sol.bool_sort();
        for (idx, prop) in comp.props.iter() {
            // Define assertion equating the proposition to its assignment
            let assign = self.prop_to_sexp(prop);
            let sexp = self
                .sol
                .define_const(Discharge::fmt_prop(idx), bs, assign)
                .unwrap();
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

        match self.check_valid(f.prop, comp) {
            None => {
                let reason = comp.add(
                    ir::Reason::misc(
                        "Discharged assumption",
                        utils::GPosIdx::UNKNOWN,
                    )
                    .into(),
                );
                Action::Change(vec![comp.assume(f.prop, reason).into()])
            }
            Some(_) => {
                let ir::Info::Assert(reason) = comp.get(f.reason) else {
                    unreachable!("expected assert reason")
                };
                self.diagnostics.push(reason.into());
                Action::Continue
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

    fn end(&mut self, _: &mut ir::Component) {
        assert!(!self.scoped, "unbalanced scopes");
        // Report all the errors
        let is_tty = atty::is(atty::Stream::Stderr);
        let writer = StandardStream::stderr(if is_tty {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
        let table = GlobalPositionTable::as_ref();
        for diag in &self.diagnostics {
            term::emit(
                &mut writer.lock(),
                &term::Config::default(),
                table.files(),
                diag,
            )
            .unwrap()
        }
    }
}
