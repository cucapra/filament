use super::CombDataflow;
use easy_smt::{self as smt, SExpr, SExprData};
use fil_ir::{self as ir, Ctx, DisplayCtx};
use fil_utils as utils;
use itertools::Itertools;
use std::{collections::HashMap, fs};

/// Minimizing goal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingGoal {
    Registers,
    Latency,
}

impl From<u64> for SchedulingGoal {
    fn from(value: u64) -> Self {
        match value {
            0 => Self::Latency,
            1 => Self::Registers,
            _ => unreachable!("Invalid scheduling goal"),
        }
    }
}

/// Struct containing information about ports in the component
struct PortInfo {
    lens: Vec<u64>,
    starts: Vec<SExpr>,
    ends: Vec<SExpr>,
}

/// Sets the proper FSM Attributes for every component
pub struct Solve<'comp> {
    /// Solver context
    sol: smt::Context,
    /// Scheduling goal
    goal: SchedulingGoal,
    /// The expression to minimize
    minimize_expr: smt::SExpr,

    /// The component to schedule
    comp: &'comp ir::Component,

    /// Map from [ir::PropIdx] to [SExpr]
    prop_map: ir::SparseInfoMap<ir::Prop, SExpr>,
    /// Map from [ir::ExprIdx] to [SExpr]
    expr_map: ir::SparseInfoMap<ir::Expr, SExpr>,
    /// Map from [ir::TimeIdx] to [SExpr]
    time_map: ir::SparseInfoMap<ir::Time, SExpr>,
    /// Let = ? parameters
    param_bind: ir::SparseInfoMap<ir::Param, SExpr>,
    /// Map from [ir::PortIdx] to [PortInfo] representing important information about each port
    port_map: ir::SparseInfoMap<ir::Port, PortInfo>,
}

impl<'comp> Solve<'comp> {
    pub fn new(
        comp: &'comp ir::Component,
        goal: u64,
        solver_file: Option<&String>,
    ) -> Self {
        // We can only schedule components with one single event!
        if comp.events().len() > 1 {
            unreachable!(
                "Components with multiple events cannot be scheduled."
            );
        }

        // We have to use Z3 as only it supports maximization of an objective function
        let (name, s_opts) = ("z3", vec!["-smt2", "-in"]);

        let mut sol = smt::ContextBuilder::new()
            .replay_file(
                solver_file.as_ref().map(|s| fs::File::create(s).unwrap()),
            )
            .solver(name, s_opts)
            .build()
            .unwrap();

        sol.push_many(1).unwrap();

        let goal = goal.into();

        Self {
            comp,
            minimize_expr: sol.numeral(0),
            goal,
            sol,
            prop_map: ir::SparseInfoMap::default(),
            expr_map: ir::SparseInfoMap::default(),
            time_map: ir::SparseInfoMap::default(),
            param_bind: ir::SparseInfoMap::default(),
            port_map: ir::SparseInfoMap::default(),
        }
    }
}

impl Solve<'_> {
    pub fn timesub_to_sexp(&self, time_sub: &ir::TimeSub) -> SExpr {
        match time_sub {
            fil_ir::TimeSub::Unit(idx) => self.expr_to_sexp(*idx),
            fil_ir::TimeSub::Sym { l, r } => {
                let l = self.time_to_sexp(*l);
                let r = self.time_to_sexp(*r);
                self.sol.sub(l, r)
            }
        }
    }

    /// Convert a time to an SExpr
    pub fn time_to_sexp(&self, time: ir::TimeIdx) -> SExpr {
        if let Some(sexpr) = self.time_map.find(time) {
            return *sexpr;
        }

        // We can ignore the event because there is only one event in this component
        let ir::Time { offset, .. } = self.comp.get(time);

        self.expr_to_sexp(*offset)
    }

    /// Fold an expression to an SExpr
    pub fn expr_to_sexp(&self, expr: ir::ExprIdx) -> SExpr {
        if let Some(sexpr) = self.expr_map.find(expr) {
            return *sexpr;
        }

        match self.comp.get(expr) {
            fil_ir::Expr::Concrete(n) => self.sol.numeral(*n),
            fil_ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr_to_sexp(*lhs);
                let rhs = self.expr_to_sexp(*rhs);
                match op {
                    fil_ast::Op::Add => self.sol.plus(lhs, rhs),
                    fil_ast::Op::Sub => self.sol.sub(lhs, rhs),
                    fil_ast::Op::Mul => self.sol.times(lhs, rhs),
                    fil_ast::Op::Div => self.sol.div(lhs, rhs),
                    fil_ast::Op::Mod => self.sol.modulo(lhs, rhs),
                }
            }
            fil_ir::Expr::If { cond, then, alt } => {
                let cond = self.prop_to_sexp(*cond);
                let then = self.expr_to_sexp(*then);
                let alt = self.expr_to_sexp(*alt);
                self.sol.ite(cond, then, alt)
            }
            fil_ir::Expr::Param(p) => {
                // The only parameters that still exist in expressions should be let = ? bindings
                // Thus, we should set them to an unknown constant value.
                self.param_bind[*p]
            }
            fil_ir::Expr::Fn { .. } => unreachable!(
                "Constraints on scheduled components do not support custom function calls."
            ),
        }
    }

    /// Fold a proposition on events to an SExpr
    pub fn prop_to_sexp(&self, prop: ir::PropIdx) -> SExpr {
        if let Some(sexpr) = self.prop_map.find(prop) {
            return *sexpr;
        }

        match self.comp.get(prop) {
            fil_ir::Prop::True => self.sol.atom("true"),
            fil_ir::Prop::False => self.sol.atom("false"),
            fil_ir::Prop::Cmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.expr_to_sexp(*lhs);
                let rhs = self.expr_to_sexp(*rhs);
                match op {
                    fil_ir::Cmp::Gt => self.sol.gt(lhs, rhs),
                    fil_ir::Cmp::Gte => self.sol.gte(lhs, rhs),
                    fil_ir::Cmp::Eq => self.sol.eq(lhs, rhs),
                }
            }
            fil_ir::Prop::TimeCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.time_to_sexp(*lhs);
                let rhs = self.time_to_sexp(*rhs);
                match op {
                    fil_ir::Cmp::Gt => self.sol.gt(lhs, rhs),
                    fil_ir::Cmp::Gte => self.sol.gte(lhs, rhs),
                    fil_ir::Cmp::Eq => self.sol.eq(lhs, rhs),
                }
            }
            fil_ir::Prop::TimeSubCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.timesub_to_sexp(lhs);
                let rhs = self.timesub_to_sexp(rhs);
                match op {
                    fil_ir::Cmp::Gt => self.sol.gt(lhs, rhs),
                    fil_ir::Cmp::Gte => self.sol.gte(lhs, rhs),
                    fil_ir::Cmp::Eq => self.sol.eq(lhs, rhs),
                }
            }
            fil_ir::Prop::Not(idx) => self.sol.not(self.prop_to_sexp(*idx)),
            fil_ir::Prop::And(idx, idx1) => self
                .sol
                .and(self.prop_to_sexp(*idx), self.prop_to_sexp(*idx1)),
            fil_ir::Prop::Or(idx, idx1) => self
                .sol
                .or(self.prop_to_sexp(*idx), self.prop_to_sexp(*idx1)),
            fil_ir::Prop::Implies(idx, idx1) => self
                .sol
                .imp(self.prop_to_sexp(*idx), self.prop_to_sexp(*idx1)),
        }
    }
}

impl Solve<'_> {
    /// Intern all conditions related to combinational delay
    pub fn combinational_delays(
        &mut self,
        comp: &ir::Component,
        ctx: &ir::Context,
    ) {
        // Generate the critical path dataflow graph
        let dataflow = CombDataflow::new(comp, ctx);

        for path in dataflow.critical_paths() {
            // If the path is length 0 or 1 this is a problem because it means
            // one single node has a combinational delay over the clock period
            if path.len() <= 1 {
                unreachable!();
            }

            // We only care about start and end of the path
            let path = path
                .iter()
                .map(|&(p, idx)| self.port(p).starts[idx as usize])
                .collect_vec()
                .into_iter();

            // It cannot be true that every port in this path is
            // scheduled (starts) on the same cycle
            let equalities = path
                .tuple_windows()
                .map(|(a, b)| self.sol.eq(a, b))
                .fold(self.sol.atom("true"), |acc, eq| self.sol.and(acc, eq));

            // Assert that this is not the case
            self.sol.assert(self.sol.not(equalities)).unwrap();
        }
    }
}

impl Solve<'_> {
    fn param_name(&self, pidx: ir::ParamIdx) -> String {
        format!("|{}@param{}|", self.comp.display(pidx), pidx.get())
    }
    /// Declare a new let = ? parameter
    fn param(&mut self, pidx: ir::ParamIdx) {
        assert!(
            !self.param_bind.contains(pidx),
            "Parameter already declared"
        );

        let sexpr = self
            .sol
            .declare_const(self.param_name(pidx), self.sol.int_sort())
            .unwrap();

        self.param_bind.push(pidx, sexpr);
    }

    fn port(&mut self, pidx: ir::PortIdx) -> &PortInfo {
        if self.port_map.contains(pidx) {
            return self.port_map.get(pidx);
        }

        let ir::Port {
            live: ir::Liveness { idxs, lens, range },
            ..
        } = self.comp.get(pidx);

        let idxs = idxs.clone();
        let ir::Range { start, end } = range.clone();

        // Lengths should be concretizable
        let lens = lens.iter().map(|l| l.concrete(self.comp)).collect_vec();

        let ranges = lens.iter().map(|&l| (0, l)).collect();

        let mut starts = Vec::new();
        let mut ends = Vec::new();
        // Loop through all the indices of the port
        for binds in utils::all_indices(ranges) {
            // temporarily add the binding to the param_bind map
            for (pidx, &bind) in idxs.iter().zip(binds.iter()) {
                self.param_bind.push(*pidx, self.sol.numeral(bind));
            }

            // Substitute the binding into start and end
            starts.push(self.time_to_sexp(start));
            ends.push(self.time_to_sexp(end));
        }

        // If this is an output port and we are scheduling for latency, add the latency
        // of each output port to the minimize expression. Multiply by width to account for
        // multiple bits
        if self.goal == SchedulingGoal::Latency
            && self.comp.get(pidx).is_sig_out()
        {
            let width = self.comp.get(pidx).width.concrete(self.comp);

            self.minimize_expr =
                ends.iter().cloned().fold(self.minimize_expr, |acc, end| {
                    self.sol
                        .plus(acc, self.sol.times(end, self.sol.numeral(width)))
                });
        }

        let info = PortInfo { lens, starts, ends };
        self.port_map.push(pidx, info);

        self.port_map.get(pidx)
    }

    /// Solve the scheduling problem
    /// Returns a binding of parameters to values
    pub fn comp(&mut self) -> ir::Bind<ir::ParamIdx, u64> {
        // First, intern all bindings for let = ? parameters and assertions/assumptions
        for cmd in &self.comp.cmds {
            match cmd {
                ir::Command::Let(l) => self.let_(l),
                ir::Command::Fact(ir::Fact { prop, .. }) => {
                    let sexpr = self.prop_to_sexp(*prop);
                    self.sol.assert(sexpr).unwrap();
                }
                fil_ir::Command::Connect(connect) => {
                    self.connect(connect);
                }
                _ => {}
            }
        }

        // Solve the scheduling problem
        let minimize = self.sol.atom("minimize");
        let expr = self.sol.list(vec![minimize, self.minimize_expr]);
        self.sol.raw_send(expr).unwrap();
        let resp = self.sol.raw_recv().unwrap();
        if resp != self.sol.atoms().success {
            unreachable!(
                "Unexpected result from solver: {}",
                self.sol.display(resp)
            );
        }

        let resp = self.sol.check().unwrap();

        if resp != smt::Response::Sat {
            unreachable!("Schedule could not be created.")
        }

        let model = self.sol.get_model().unwrap();
        log::trace!("Model solution: {}", self.sol.display(model));

        let SExprData::List(bindings) = self.sol.get(model) else {
            unreachable!(
                "Expected model to be a list of bindings, got {}",
                self.sol.display(model)
            )
        };

        let bindings: HashMap<String, u64> = bindings
            .iter()
            .map(|binding| {
                let exprs = match self.sol.get(*binding) {
                    SExprData::List(exprs) => exprs,
                    _ => unreachable!(
                        "Binding {} was not a list",
                        self.sol.display(*binding)
                    ),
                };

                let exprs: [SExpr; 5] = exprs.try_into().unwrap_or_else(|_| {
                    panic!(
                        "Binding {} had invalid number of expressions",
                        self.sol.display(*binding)
                    )
                });

                // The format of the binding should be
                // (define-fun <name> () Int <value>)
                assert!(exprs[0] == self.sol.atoms().define_fun);
                assert!(matches!(self.sol.get(exprs[2]), SExprData::List(&[])));
                assert!(exprs[3] == self.sol.atoms().int);

                let SExprData::Atom(name) = self.sol.get(exprs[1]) else {
                    unreachable!(
                        "Expected name to be an atom, got {}",
                        self.sol.display(exprs[1])
                    )
                };

                let SExprData::Atom(value) = self.sol.get(exprs[4]) else {
                    unreachable!(
                        "Expected value to be an atom, got {}",
                        self.sol.display(exprs[4])
                    )
                };
                let value = value.parse::<u64>().unwrap();

                (name.to_string(), value)
            })
            .collect();

        ir::Bind::new(self.comp.cmds.iter().filter_map(|cmd| match cmd {
            ir::Command::Let(ir::Let { param, .. }) => {
                let value = *bindings.get(&self.param_name(*param)).unwrap();
                Some((*param, value))
            }
            _ => None,
        }))
    }

    fn let_(&mut self, l: &ir::Let) {
        let ir::Let { param, expr } = l;
        // Expr should be none, as monomorphization should have already removed all other let bindings
        assert!(
            expr.is_none(),
            "Found let binding with non-? expression when scheduling."
        );
        self.param(*param);
    }

    fn connect(&mut self, con: &ir::Connect) {
        let ir::Connect {
            src:
                ir::Access {
                    port: src_port,
                    ranges: src_ranges,
                },
            dst:
                ir::Access {
                    port: dst_port,
                    ranges: dst_ranges,
                },
            ..
        } = con;

        let src_ranges = src_ranges
            .iter()
            .map(|(s, e)| (s.concrete(self.comp), e.concrete(self.comp)))
            .collect_vec();

        let dst_ranges = dst_ranges
            .iter()
            .map(|(s, e)| (s.concrete(self.comp), e.concrete(self.comp)))
            .collect_vec();

        let start_info = self.port(*src_port);

        let start_exprs = utils::all_indices(src_ranges)
            .into_iter()
            .map(|multi_idx| utils::flat_idx(&multi_idx, &start_info.lens))
            .map(|idx| {
                (
                    start_info.starts[idx as usize],
                    start_info.ends[idx as usize],
                )
            })
            .collect_vec();

        let end_info = self.port(*dst_port);
        let end_exprs = utils::all_indices(dst_ranges)
            .into_iter()
            .map(|multi_idx| utils::flat_idx(&multi_idx, &end_info.lens))
            .map(|idx| {
                (end_info.starts[idx as usize], end_info.ends[idx as usize])
            })
            .collect_vec();

        let width = self.comp.get(*src_port).width.concrete(self.comp);

        log::trace!("Connecting {} to {}", src_port, dst_port);

        // For each actual pair of ports:
        for ((src_start, src_end), (dst_start, dst_end)) in
            start_exprs.into_iter().zip(end_exprs)
        {
            // The destination port must happen after the source port
            self.sol.assert(self.sol.lte(src_start, dst_start)).unwrap();

            // We can create a register that will extend the lifetime of the source port to the destination port. Given a src port valid from [a, b], and a dest port from [c, d], we need a register that holds from [b-1, d].
            // The number of FFs necessary to do this is thus d - b
            let reg_expr = self.sol.sub(dst_end, src_end);
            // reg_expr cannot be negative
            let reg_expr = self.sol.ite(
                self.sol.gte(reg_expr, self.sol.numeral(0)),
                reg_expr,
                self.sol.numeral(0),
            );

            // multiply by the width of the port
            let reg_expr = self.sol.times(reg_expr, self.sol.numeral(width));

            // add this to the minimize expression
            if self.goal == SchedulingGoal::Registers {
                self.minimize_expr =
                    self.sol.plus(self.minimize_expr, reg_expr);
            }
        }
    }
}
