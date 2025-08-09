use crate::cmdline;
use crate::ir_visitor::{Action, Construct, Visitor, VisitorData};
use crate::log_time;
use crate::utils::HoistFacts;
use codespan_reporting::diagnostic::Diagnostic;
use codespan_reporting::{diagnostic as cr, term};
use easy_smt as smt;
use fil_ast as ast;
use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils::GlobalPositionTable;
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::{fs, iter};
use term::termcolor::{ColorChoice, StandardStream};

// We need this so we can implement Default for SExprs, allowing us to push to the prop_map out of order, when handling if-exprs
#[derive(Clone, Copy)]
enum SExprWrapper {
    SExpr(easy_smt::SExpr),
    SExprEmpty,
}

impl Default for SExprWrapper {
    fn default() -> Self {
        Self::SExprEmpty
    }
}

impl SExprWrapper {
    fn get(&self) -> easy_smt::SExpr {
        match self {
            SExprWrapper::SExpr(s) => *s,
            SExprWrapper::SExprEmpty => {
                panic!("don't call this on empty SExprWrapper")
            }
        }
    }
}

#[derive(Default)]
struct Assign(Vec<(ir::ParamIdx, String)>);

impl Assign {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn display(&self, ctx: &ir::Component) -> String {
        self.0
            .iter()
            .filter_map(|(k, v)| {
                // Attempt to parse value as a number
                match v.parse::<i64>() {
                    Ok(0) => None,
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
    /// Are we using a bitvector encoding
    bv_size: Option<u8>,
    /// Which solver are we using
    sol_base: cmdline::Solver,
    /// Defined global functions
    func_map: HashMap<ast::Fn, smt::SExpr>,
    /// Defined functions for `some` parameters on components
    comp_param_map: HashMap<ir::Foreign<ir::Param, ir::Component>, smt::SExpr>,

    // Defined names. These are sparse in case certain parameters or events have been invalidated.
    param_map: ir::SparseInfoMap<ir::Param, smt::SExpr>,
    ev_map: ir::SparseInfoMap<ir::Event, smt::SExpr>,
    // Composite expressions
    expr_map: ir::SparseInfoMap<ir::Expr, smt::SExpr>,
    time_map: ir::SparseInfoMap<ir::Time, smt::SExpr>,
    // Propositions
    prop_map: ir::DenseIndexInfo<ir::Prop, SExprWrapper>,
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
                ("z3", vec!["-smt2", "-in"])
            }
            cmdline::Solver::Boolector => {
                log::debug!("Using boolector solver");
                ("boolector", vec!["--incremental"])
            }
            cmdline::Solver::CVC5 => {
                log::debug!("Using cvc5 solver");
                ("cvc5", vec!["--incremental", "--force-logic=ALL"])
            }
            cmdline::Solver::Bitwuzla => {
                log::debug!("Using bitwuzla solver");
                ("bitwuzla", vec![])
            }
        };
        smt::ContextBuilder::new()
            .replay_file(
                opts.solver_replay_file
                    .as_ref()
                    .map(|s| fs::File::create(s).unwrap()),
            )
            .solver(name)
            .solver_args(s_opts)
            .build()
            .unwrap()
    }

    fn app(&mut self, f: smt::SExpr, args: Vec<smt::SExpr>) -> smt::SExpr {
        if args.is_empty() {
            f
        } else {
            self.sol.list(iter::once(f).chain(args).collect_vec())
        }
    }

    /// Return the sort to be used in this encoding.
    /// When using bv encoding, we return twice the number of bits provided on
    /// the command line so that we can encode overflow checks.
    #[inline]
    fn bv_size(&self) -> Option<u8> {
        self.bv_size.map(|v| v * 2)
    }

    fn sort(&self) -> smt::SExpr {
        if let Some(v) = self.bv_size() {
            self.sol.bit_vec_sort(self.sol.numeral(v))
        } else {
            self.sol.int_sort()
        }
    }
    fn num(&self, n: u64) -> smt::SExpr {
        if let Some(v) = self.bv_size() {
            self.sol.binary(v as usize, n)
        } else {
            self.sol.numeral(n)
        }
    }
    fn plus(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvadd(l, r)
        } else {
            self.sol.plus(l, r)
        }
    }
    fn sub(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvsub(l, r)
        } else {
            self.sol.sub(l, r)
        }
    }
    fn times(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvmul(l, r)
        } else {
            self.sol.times(l, r)
        }
    }
    fn div(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvudiv(l, r)
        } else {
            self.sol.div(l, r)
        }
    }
    fn modulo(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvurem(l, r)
        } else {
            self.sol.modulo(l, r)
        }
    }
    fn gt(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvugt(l, r)
        } else {
            self.sol.gt(l, r)
        }
    }
    fn gte(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        if self.bv_size.is_some() {
            self.sol.bvuge(l, r)
        } else {
            self.sol.gte(l, r)
        }
    }
    fn eq(&self, l: smt::SExpr, r: smt::SExpr) -> smt::SExpr {
        self.sol.eq(l, r)
    }
    /// Assert that the expression is not overflowing
    /// e >= 0 && e < 2^bvsize
    fn overflow_assert(&mut self, e: smt::SExpr) {
        let Some(v) = self.bv_size else {
            return;
        };
        let max = self.num((1 << v) - 1);
        let zero = self.num(0);
        let ge_zero = self.gte(e, zero);
        let lt_max = self.gt(max, e);
        let and = self.sol.and(ge_zero, lt_max);
        self.sol.assert(and).unwrap();
    }
}

impl Construct for Discharge {
    fn from(opts: &cmdline::Opts, ctx: &mut ir::Context) -> Self {
        let mut out = Self {
            bv_size: opts.solver_bv,
            sol: Self::conf_solver(opts),
            sol_base: opts.solver,
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
            comp_param_map: Default::default(),
        };

        out.define_funcs();

        // For each `some` parameter of a component, define function from the
        // input parameters of the component to the `some` parameter.
        for (comp_idx, comp) in ctx.comps.iter() {
            let num_args = comp.param_args().len();
            for some_param in comp.exist_params() {
                let ir::ParamOwner::Exists { opaque } = &comp[some_param].owner
                else {
                    unreachable!()
                };
                if *opaque {
                    // If this is an opaque parameter, then we don't define the function
                    continue;
                }
                let func = out
                    .sol
                    .declare_fun(
                        format!(
                            "comp{}_param{}",
                            comp_idx.get(),
                            some_param.get()
                        ),
                        (0..num_args).map(|_| out.sort()).collect_vec(),
                        out.sort(),
                    )
                    .unwrap();
                let f = ir::Foreign::new(some_param, comp_idx);
                out.comp_param_map.insert(f, func);
            }
        }

        out.sol.push_many(1).unwrap();
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
        self.sol.pop_many(1).unwrap();
        self.sol.push_many(1).unwrap();
    }
}

impl Discharge {
    fn fmt_param(&self, param: ir::ParamIdx, ctx: &ir::Component) -> String {
        match self.sol_base {
            // CVC5 does not correctly print out quoted SExps
            cmdline::Solver::Z3 => {
                format!("|{}@param{}|", ctx.display(param), param.get())
            }
            _ => {
                format!("param{}", param.get())
            }
        }
    }

    fn fmt_event(&self, event: ir::EventIdx, ctx: &ir::Component) -> String {
        match self.sol_base {
            cmdline::Solver::Z3 => {
                format!("|{}@event{}|", ctx.display(event), event.get())
            }
            _ => format!("event{}", event.get()),
        }
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
        let is = self.sort();

        macro_rules! sol_fn(
            ($name:tt($($args:ident),*) -> $out:ident) => {
                self.func_map.insert(ast::Fn::$name, self
                    .sol
                    .declare_fun(stringify!($name).to_lowercase(), vec![$($args),*], $out)
                    .unwrap());
            }
        );

        self.func_map = Default::default();

        sol_fn!(Pow2(is) -> is);
        sol_fn!(Log2(is) -> is);
        sol_fn!(SinB(is, is) -> is);
        sol_fn!(CosB(is, is) -> is);
        sol_fn!(BitRev(is, is) -> is);
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
                log::debug!("{} -> {}", self.sol.display(s), p);
                rev_map.insert(s, *p);
                s
            })
            .collect_vec();
        let num_vars = sexps.len();

        let model = self.sol.get_value(sexps).unwrap();
        assert!(
            model.len() == num_vars,
            "{num_vars} relevant variables but the model contains assignments for {} variables",
            model.len()
        );

        Assign(
            model
                .into_iter()
                .flat_map(|(p, v)| {
                    let Some(&p) = rev_map.get(&p) else {
                        unreachable!(
                            "missing binding for sexp {}",
                            self.sol.display(p)
                        );
                    };
                    Some((p, self.sol.display(v).to_string()))
                })
                .collect_vec(),
        )
    }

    /// Check whether the proposition is valid.
    /// Adds an error to the diagnositcs reporter if the proposition cannot be proved.
    fn check_valid(&mut self, fact: ir::Fact, ctx: &ir::Component) {
        let prop = fact.prop;
        #[allow(clippy::map_entry)]
        if !self.checked.contains_key(&prop) {
            let actlit = self.new_act_lit();
            let sexp = self.prop_map[prop];
            let imp = self.sol.imp(actlit, self.sol.not(sexp.get()));
            self.sol.assert(imp).unwrap();
            // Disable the activation literal
            log::debug!("Checking {}", ctx.display(prop.consequent(ctx)));
            let res = log_time!(
                self.sol.check_assuming([actlit]).unwrap(),
                ctx.display(prop.consequent(ctx));
                100
            );
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
            // Deassert the actlit after the `get-model` call.
            self.sol.assert(self.sol.not(actlit)).unwrap();
            self.checked.insert(prop, out);
        }
        if let Some(assign) = &self.checked[&prop] {
            let Some(ir::info::Assert(reason)) =
                ctx.get(fact.reason).as_assert()
            else {
                // No information was given on who generated this error
                let diag = Diagnostic::error().with_notes(vec![
                    format!(
                        "Cannot prove constraint: {}",
                        ctx.display(fact.prop.consequent(ctx))
                    ),
                    "No information was given on who generated this error. Please report this as a bug in the compiler with the program that triggered it."
                        .to_string(),
                ]);
                self.diagnostics.push(diag);
                return;
            };
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
        match expr {
            ir::Expr::Param(p) => self.param_map[*p],
            ir::Expr::Concrete(n) => self.num(*n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let l = self.expr_map[*lhs];
                let r = self.expr_map[*rhs];
                match op {
                    ast::Op::Add => self.plus(l, r),
                    ast::Op::Sub => self.sub(l, r),
                    ast::Op::Mul => self.times(l, r),
                    ast::Op::Div => self.div(l, r),
                    ast::Op::Mod => self.modulo(l, r),
                }
            }
            ir::Expr::Fn { op, args } => {
                let args = args.iter().map(|e| self.expr_map[*e]);
                self.sol.list(
                    iter::once(self.func_map[op]).chain(args).collect_vec(),
                )
            }
            ir::Expr::If { cond, then, alt } => {
                let then = self.expr_map[*then];
                let alt = self.expr_map[*alt];
                let cond = self.prop_map[*cond];
                self.sol.ite(cond.get(), then, alt)
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
            ir::Cmp::Gt => self.gt(l, r),
            ir::Cmp::Gte => self.gte(l, r),
            ir::Cmp::Eq => self.eq(l, r),
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
                        ctx.sub(l, r)
                    }
                })
            }
            ir::Prop::Not(p) => sol.not(self.prop_map[*p].get()),
            ir::Prop::And(l, r) => {
                let l = self.prop_map[*l].get();
                let r = self.prop_map[*r].get();
                sol.and(l, r)
            }
            ir::Prop::Or(l, r) => {
                let l = self.prop_map[*l].get();
                let r = self.prop_map[*r].get();
                sol.or(l, r)
            }
            ir::Prop::Implies(l, r) => {
                let l = self.prop_map[*l].get();
                let r = self.prop_map[*r].get();
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
        self.to_prove = HoistFacts::hoist(&mut data.comp);

        for fact in &self.to_prove {
            log::debug!("Checking {}", data.comp.display(fact.prop));
        }

        let comp = &data.comp;
        // Declare all parameters
        let int = self.sort();
        for (idx, _) in data.comp.params().iter() {
            let sexp = self
                .sol
                .declare_fun(self.fmt_param(idx, comp), vec![], int)
                .unwrap();
            self.overflow_assert(sexp);
            self.param_map.push(idx, sexp);
        }

        // Declare all events
        for (idx, _) in data.comp.events().iter() {
            let sexp = self
                .sol
                .declare_fun(self.fmt_event(idx, comp), vec![], int)
                .unwrap();
            self.overflow_assert(sexp);
            self.ev_map.push(idx, sexp);
        }

        let bs = self.sol.bool_sort();

        let mut interned_props = HashSet::new();

        // Declare all expressions
        for (idx, expr) in data
            .comp
            .exprs()
            .iter()
            .filter(|(idx, _)| idx.valid(&data.comp))
        {
            // do props inside of exprs ahead of time
            let relevant_props = idx
                .relevant_props(&data.comp)
                .into_iter()
                .map(|i| (i, data.comp.get(i)));

            for (pidx, prop) in relevant_props {
                // Save the proposition in the interned_props set
                interned_props.insert(pidx);

                let assign = self.prop_to_sexp(prop);
                let sexp = self
                    .sol
                    .define_const(Discharge::fmt_prop(pidx), bs, assign)
                    .unwrap();
                self.prop_map.insert(pidx, SExprWrapper::SExpr(sexp));
            }

            let assign = self.expr_to_sexp(expr);
            let sexp = self
                .sol
                .define_const(Self::fmt_expr(idx), int, assign)
                .unwrap();
            self.overflow_assert(sexp);
            self.expr_map.push(idx, sexp);
        }

        // Assert bindings for all let-bound parameters
        for (idx, p) in data.comp.params().iter() {
            let ir::ParamOwner::Let { bind: Some(bind) } = &p.owner else {
                continue;
            };
            let param_s = self.param_map[idx];
            let bind_s = self.expr_map[*bind];
            let assign = self.sol.eq(param_s, bind_s);
            self.sol.assert(assign).unwrap();
        }

        // Declare all time expressions
        for (idx, ir::Time { event, offset }) in data
            .comp
            .times()
            .iter()
            .filter(|(idx, _)| idx.valid(&data.comp))
        {
            if self.ev_map.contains(*event) && self.expr_map.contains(*offset) {
                let assign =
                    self.plus(self.ev_map[*event], self.expr_map[*offset]);
                let sexp = self
                    .sol
                    .define_const(Self::fmt_time(idx), int, assign)
                    .unwrap();
                self.overflow_assert(sexp);
                self.time_map.push(idx, sexp);
            }
        }

        // Declare all propositions
        for (idx, prop) in data
            .comp
            .props()
            .iter()
            .filter(|(idx, _)| idx.valid(&data.comp))
            // Filter out propositions that are already defined in expressions
            .filter(|(idx, _)| !interned_props.contains(idx))
        {
            // Define assertion equating the proposition to its assignment
            let assign = self.prop_to_sexp(prop);
            if let Ok(sexp) =
                self.sol.define_const(Discharge::fmt_prop(idx), bs, assign)
            {
                self.prop_map.insert(idx, SExprWrapper::SExpr(sexp));
            }
        }
        // Pass does not need to traverse the control program.
        Action::Continue
    }

    fn instance(&mut self, idx: ir::InstIdx, data: &mut VisitorData) -> Action {
        let comp = &data.comp;
        let inst = &comp[idx];
        let sexp_args =
            inst.args.iter().map(|e| self.expr_map[*e]).collect_vec();
        for param in &inst.params {
            let ir::ParamOwner::Instance { base, .. } = &comp[*param].owner
            else {
                unreachable!()
            };
            // If the parameter is not opaque, we can assert that it is equal to the value of the function
            if let Some(f) = self.comp_param_map.get(base) {
                let param_s = self.param_map[*param];
                let app = self.app(*f, sexp_args.clone());
                let assign = self.sol.eq(param_s, app);
                self.sol.assert(assign).unwrap();
            }
        }
        Action::Continue
    }

    fn end(&mut self, data: &mut VisitorData) {
        if self.to_prove.is_empty() {
            return;
        }

        if !data.opts.discharge_separate {
            // Attempt to prove all facts
            let total_prop = self.sol.and_many(
                self.to_prove.iter().map(|f| self.prop_map[f.prop].get()),
            );
            let total_prop = self.sol.not(total_prop);
            self.sol.assert(total_prop).unwrap();

            // If there is at least one failing prop, roll back to individually checking the props for error reporting
            if matches!(self.sol.check().unwrap(), smt::Response::Sat) {
                log::info!(
                    "Failed to prove all facts. Checking each fact individually"
                );
                self.failing_props(&data.comp);
            }
        } else {
            // Check each proposition individually
            let to_prove = std::mem::take(&mut self.to_prove);
            for fact in to_prove {
                self.check_valid(fact, &data.comp);
            }
        }

        // Report all the errors
        let is_tty = atty::is(atty::Stream::Stderr);
        let writer = StandardStream::stderr(if is_tty {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
        let table = GlobalPositionTable::get();
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
