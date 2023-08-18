use crate::ir::{Ctx, DisplayCtx};
use crate::ir_visitor::{Action, Construct, Visitor, VisitorData};
use crate::utils::GlobalPositionTable;
use crate::{ast, cmdline, ir, log_time};
use codespan_reporting::{diagnostic as cr, term};
use easy_smt as smt;
use itertools::Itertools;
use std::collections::HashMap;
use std::{fs, iter};
use term::termcolor::{ColorChoice, StandardStream};

#[derive(Default)]
pub struct Assign(Vec<(ir::ParamIdx, String)>);

impl Assign {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn display(&self, ctx: &ir::Component) -> String {
        self.0
            .iter()
            .filter_map(|(k, v)| {
                // Attempt to parse value as a number
                match v.parse::<u64>() {
                    Ok(v) if v == 0 => None,
                    _ => Some(format!("{} = {v}", ctx.display(*k))),
                }
            })
            .join(", ")
    }
}

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

    // counter for activation literals generated
    act_lit_count: u32,

    /// Report the unsatisfied constraint and generate a model
    show_models: bool,

    to_prove: Vec<ir::Fact>,

    // Diagnostics to be reported
    diagnostics: Vec<cr::Diagnostic<usize>>,
    /// Number of errors encountered
    error_count: u64,
}

impl Discharge {
    /// Configure solver to use in this pass
    fn conf_solver(opts: &cmdline::Opts) -> smt::Context {
        let (name, s_opts) = match opts.solver {
            cmdline::Solver::Z3 => {
                log::debug!("Using z3 solver");
                ("z3", &["-smt2", "-in"])
            }
            cmdline::Solver::CVC5 => {
                log::debug!("Using cvc5 solver");
                ("cvc5", &["--incremental", "--force-logic=ALL"])
            }
        };
        smt::ContextBuilder::new()
            .replay_file(
                opts.solver_replay_file
                    .as_ref()
                    .map(|s| fs::File::create(s).unwrap()),
            )
            .solver(name, s_opts)
            .build()
            .unwrap()
    }
}

impl Construct for Discharge {
    fn from(opts: &cmdline::Opts, _: &mut ir::Context) -> Self {
        let mut out = Self {
            sol: Self::conf_solver(opts),
            scoped: false,
            error_count: 0,
            act_lit_count: 0,
            to_prove: vec![],
            show_models: opts.show_models,
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
        out.sol.push().unwrap();
        out
    }

    fn clear_data(&mut self) {
        self.param_map.clear();
        self.prop_map.clear();
        self.time_map.clear();
        self.ev_map.clear();
        self.expr_map.clear();
        self.checked.clear();
        self.diagnostics.clear();
        self.act_lit_count = 0;
        self.to_prove.clear();

        // Create a new solver context
        self.sol.pop().unwrap();
        self.sol.push().unwrap();
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

    fn new_act_lit(&mut self) -> smt::SExpr {
        self.act_lit_count += 1;
        self.sol
            .declare_const(
                format!("act_lit{}", self.act_lit_count),
                self.sol.bool_sort(),
            )
            .unwrap()
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
        if relevant_vars.is_empty() {
            return Assign(vec![]);
        }
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

        Assign(
            self.sol
                .get_value(sexps)
                .unwrap()
                .into_iter()
                .map(|(p, v)| {
                    let p = rev_map[&p];
                    (p, self.sol.display(v).to_string())
                })
                .collect_vec(),
        )
    }

    /// Check whether the proposition is valid.
    /// Returns a set of assignments if the proposition is not valid.
    fn check_valid(&mut self, fact: ir::Fact, ctx: &ir::Component) {
        let prop = fact.prop;
        #[allow(clippy::map_entry)]
        if !self.checked.contains_key(&prop) {
            let actlit = self.new_act_lit();
            let sexp = self.prop_map[prop];
            let imp = self.sol.imp(actlit, self.sol.not(sexp));
            self.sol.assert(imp).unwrap();
            // Disable the activation literal
            let res = log_time!(
                self.sol.check_assuming([actlit]).unwrap(),
                ctx.display(prop.consequent(ctx));
                100
            );
            self.sol.assert(self.sol.not(actlit)).unwrap();
            let out = match res {
                smt::Response::Sat => {
                    if self.show_models {
                        Some(self.get_assignments(
                            ctx.prop_params(prop.consequent(ctx)),
                        ))
                    } else {
                        Some(Assign::default())
                    }
                }
                smt::Response::Unsat => None,
                smt::Response::Unknown => panic!("Solver returned unknown"),
            };
            self.checked.insert(prop, out);
        }
        if let Some(assign) = &self.checked[&prop] {
            let ir::info::Assert(reason) =
                ctx.get(fact.reason).as_assert().unwrap();
            let mut diag = reason.diag(ctx);
            if self.show_models {
                diag = reason.diag(ctx).with_notes(vec![format!(
                    "Cannot prove constraint: {}",
                    ctx.display(fact.prop.consequent(ctx))
                )]);
                if !assign.is_empty() {
                    diag = diag.with_notes(vec![format!(
                        "Counterexample: {} (unmentioned parameters are 0)",
                        assign.display(ctx)
                    )]);
                }
            }
            self.diagnostics.push(diag);
        }
    }

    /// Find the failing facts from the given component and add diagnostics for them
    fn failing_props(&mut self, comp: &ir::Component) {
        let props = std::mem::take(&mut self.to_prove);
        for fact in props {
            self.check_valid(fact, comp);
        }
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
    fn name() -> &'static str {
        "discharge"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        // Declare all parameters
        let int = self.sol.int_sort();
        for (idx, _) in data.comp.params().iter() {
            let sexp = self
                .sol
                .declare_fun(Self::fmt_param(idx), vec![], int)
                .unwrap();
            self.param_map.push(idx, sexp);
        }

        // Declare all events
        for (idx, _) in data.comp.events().iter() {
            let sexp = self
                .sol
                .declare_fun(Self::fmt_event(idx), vec![], int)
                .unwrap();
            self.ev_map.push(idx, sexp);
        }

        // Declare all expressions
        for (idx, expr) in data.comp.exprs().iter() {
            let assign = self.expr_to_sexp(expr);
            let sexp = self
                .sol
                .define_const(Self::fmt_expr(idx), int, assign)
                .unwrap();
            self.expr_map.push(idx, sexp);
        }

        // Declare all time expressions
        for (idx, ir::Time { event, offset }) in data.comp.times().iter() {
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
        for (idx, prop) in data.comp.props().iter() {
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

    fn fact(&mut self, f: &mut ir::Fact, _: &mut VisitorData) -> Action {
        if self.scoped {
            panic!("scoped facts not supported. Run `hoist-facts` before this pass");
        }

        if f.is_assume() {
            panic!(
                "assumptions should have been eliminated by `hoist-facts` pass"
            )
        }

        // Defer proof obligations till the end of the component pass
        self.to_prove.push(f.clone());
        Action::Continue
    }

    fn do_if(&mut self, i: &mut ir::If, data: &mut VisitorData) -> Action {
        let orig = self.scoped;
        self.scoped = true;
        let out = self
            .visit_cmds(&mut i.then, data)
            .and_then(|| self.visit_cmds(&mut i.alt, data));
        self.scoped = orig;
        out
    }

    fn do_loop(&mut self, l: &mut ir::Loop, data: &mut VisitorData) -> Action {
        let orig = self.scoped;
        self.scoped = true;
        let out = self
            .start_loop(l, data)
            .and_then(|| self.visit_cmds(&mut l.body, data));
        self.scoped = orig;
        out
    }

    fn end(&mut self, data: &mut VisitorData) {
        assert!(!self.scoped, "unbalanced scopes");

        if self.to_prove.is_empty() {
            return;
        }

        // Attempt to prove all facts
        let total_prop = self
            .sol
            .and_many(self.to_prove.iter().map(|f| self.prop_map[f.prop]));
        let total_prop = self.sol.not(total_prop);
        self.sol.assert(total_prop).unwrap();

        // If there is at least one failing prop, roll back to individually checking the props for error reporting
        if matches!(self.sol.check().unwrap(), smt::Response::Sat) {
            self.failing_props(&data.comp);
        }

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
            .unwrap();
            self.error_count += 1;
        }
    }

    fn after_traversal(&mut self) -> Option<u64> {
        if self.error_count > 0 {
            Some(self.error_count)
        } else {
            None
        }
    }
}
