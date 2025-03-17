use super::CombDataflow;
use crate::ir_visitor::{Action, Construct, Visitor, VisitorData};
use easy_smt::{self as smt, SExpr, SExprData};
use fil_ir::{self as ir, AddCtx, Ctx, DisplayCtx, MutCtx};
use fil_utils::{AttrCtx, CompNum};
use itertools::Itertools;
use std::{collections::HashMap, fs};

/// Minimizing goal
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

/// Sets the proper FSM Attributes for every component
pub struct Solve {
    /// Solver context
    sol: smt::Context,
    /// Scheduling goal
    goal: SchedulingGoal,
    /// The expression to minimize
    minimize_expr: smt::SExpr,
    // /// Map from ir elements to SExprs
    // expr_map: ir::SparseInfoMap<ir::Expr, SExpr>,
    // port_map: ir::SparseInfoMap<ir::PortIdx, SExpr>,
    // event_map: ir::SparseInfoMap<ir::EventIdx, SExpr>,
    // time_map: ir::SparseInfoMap<ir::TimeIdx, SExpr>,
    // prop_map: ir::SparseInfoMap<ir::PropIdx, SExpr>,
}

impl Solve {
    /// Get the constant names for the start and end of a port
    pub fn get_port_name(&self, pidx: ir::PortIdx) -> (String, String) {
        (
            format!("port{}_s", pidx.get()),
            format!("port{}_e", pidx.get()),
        )
    }

    /// Get the constant name for the time of an event
    pub fn get_evt_name(
        &self,
        inv: ir::InvIdx,
        evt: ir::Foreign<ir::Event, ir::Component>,
    ) -> String {
        format!("inv{}ev{}", inv.get(), evt.key().get())
    }

    /// Get the SExprs for the start and end of a port
    pub fn get_port(&self, pidx: ir::PortIdx) -> (smt::SExpr, smt::SExpr) {
        let (start, end) = self.get_port_name(pidx);
        (self.sol.atom(start), self.sol.atom(end))
    }

    /// Get the SExpr for the time of an event
    pub fn get_inv_evt(
        &self,
        inv: ir::InvIdx,
        evt: ir::Foreign<ir::Event, ir::Component>,
    ) -> smt::SExpr {
        self.sol.atom(self.get_evt_name(inv, evt))
    }

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
            let path = path.iter().map(|p| self.get_port(*p).0);

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

    pub fn timesub_to_sexp(
        &self,
        ctx: &ir::Component,
        event_bind: &ir::SparseInfoMap<ir::Event, SExpr>,
        time_sub: &ir::TimeSub,
    ) -> SExpr {
        match time_sub {
            fil_ir::TimeSub::Unit(idx) => {
                self.expr_to_sexp(ctx, event_bind, *idx)
            }
            fil_ir::TimeSub::Sym { l, r } => {
                let l = self.time_to_sexp(ctx, event_bind, *l);
                let r = self.time_to_sexp(ctx, event_bind, *r);
                self.sol.sub(l, r)
            }
        }
    }

    pub fn time_to_sexp(
        &self,
        ctx: &ir::Component,
        event_bind: &ir::SparseInfoMap<ir::Event, SExpr>,
        time: ir::TimeIdx,
    ) -> SExpr {
        let ir::Time { event, offset } = ctx.get(time);

        let offset = self.expr_to_sexp(ctx, event_bind, *offset);
        let event = *event_bind.get(*event);

        self.sol.plus(event, offset)
    }

    /// Fold an expression to an SExpr
    pub fn expr_to_sexp(
        &self,
        ctx: &ir::Component,
        event_bind: &ir::SparseInfoMap<ir::Event, SExpr>,
        expr: ir::ExprIdx,
    ) -> SExpr {
        match ctx.get(expr) {
            fil_ir::Expr::Concrete(n) => self.sol.numeral(*n),
            fil_ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr_to_sexp(ctx, event_bind, *lhs);
                let rhs = self.expr_to_sexp(ctx, event_bind, *rhs);
                match op {
                    fil_ast::Op::Add => self.sol.plus(lhs, rhs),
                    fil_ast::Op::Sub => self.sol.sub(lhs, rhs),
                    fil_ast::Op::Mul => self.sol.times(lhs, rhs),
                    fil_ast::Op::Div => self.sol.div(lhs, rhs),
                    fil_ast::Op::Mod => self.sol.modulo(lhs, rhs),
                }
            }
            fil_ir::Expr::If { cond, then, alt } => {
                let cond = self.prop_to_sexp(ctx, event_bind, *cond);
                let then = self.expr_to_sexp(ctx, event_bind, *then);
                let alt = self.expr_to_sexp(ctx, event_bind, *alt);
                self.sol.ite(cond, then, alt)
            }
            fil_ir::Expr::Fn { .. } => unreachable!(
                "Constraints on scheduled components do not support custom function calls."
            ),
            fil_ir::Expr::Param(_) => {
                unreachable!("Parameters should have been monomorphized")
            }
        }
    }

    /// Fold a proposition on events to an SExpr
    pub fn prop_to_sexp(
        &self,
        ctx: &ir::Component,
        event_bind: &ir::SparseInfoMap<ir::Event, SExpr>,
        prop: ir::PropIdx,
    ) -> SExpr {
        match ctx.get(prop) {
            fil_ir::Prop::True => self.sol.atom("true"),
            fil_ir::Prop::False => self.sol.atom("false"),
            fil_ir::Prop::Cmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.expr_to_sexp(ctx, event_bind, *lhs);
                let rhs = self.expr_to_sexp(ctx, event_bind, *rhs);
                match op {
                    fil_ir::Cmp::Gt => self.sol.gt(lhs, rhs),
                    fil_ir::Cmp::Gte => self.sol.gte(lhs, rhs),
                    fil_ir::Cmp::Eq => self.sol.eq(lhs, rhs),
                }
            }
            fil_ir::Prop::TimeCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.time_to_sexp(ctx, event_bind, *lhs);
                let rhs = self.time_to_sexp(ctx, event_bind, *rhs);
                match op {
                    fil_ir::Cmp::Gt => self.sol.gt(lhs, rhs),
                    fil_ir::Cmp::Gte => self.sol.gte(lhs, rhs),
                    fil_ir::Cmp::Eq => self.sol.eq(lhs, rhs),
                }
            }
            fil_ir::Prop::TimeSubCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.timesub_to_sexp(ctx, event_bind, lhs);
                let rhs = self.timesub_to_sexp(ctx, event_bind, rhs);
                match op {
                    fil_ir::Cmp::Gt => self.sol.gt(lhs, rhs),
                    fil_ir::Cmp::Gte => self.sol.gte(lhs, rhs),
                    fil_ir::Cmp::Eq => self.sol.eq(lhs, rhs),
                }
            }
            fil_ir::Prop::Not(idx) => {
                self.sol.not(self.prop_to_sexp(ctx, event_bind, *idx))
            }
            fil_ir::Prop::And(idx, idx1) => self.sol.and(
                self.prop_to_sexp(ctx, event_bind, *idx),
                self.prop_to_sexp(ctx, event_bind, *idx1),
            ),
            fil_ir::Prop::Or(idx, idx1) => self.sol.or(
                self.prop_to_sexp(ctx, event_bind, *idx),
                self.prop_to_sexp(ctx, event_bind, *idx1),
            ),
            fil_ir::Prop::Implies(idx, idx1) => self.sol.imp(
                self.prop_to_sexp(ctx, event_bind, *idx),
                self.prop_to_sexp(ctx, event_bind, *idx1),
            ),
        }
    }
}

impl Construct for Solve {
    fn from(opts: &crate::cmdline::Opts, _: &mut ir::Context) -> Self {
        // We have to use Z3 as only it supports maximization of an objective function
        let (name, s_opts) = ("z3", vec!["-smt2", "-in"]);

        let mut sol = smt::ContextBuilder::new()
            .replay_file(
                opts.solver_replay_file
                    .as_ref()
                    .map(|s| fs::File::create(s).unwrap()),
            )
            .solver(name, s_opts)
            .build()
            .unwrap();

        sol.push_many(1).unwrap();

        Self {
            minimize_expr: sol.numeral(0),
            goal: SchedulingGoal::Latency,
            sol,
        }
    }

    fn clear_data(&mut self) {
        // Create a new solver context
        self.sol.pop_many(1).unwrap();
        self.sol.push_many(1).unwrap();
    }
}

impl Visitor for Solve {
    fn name() -> &'static str {
        "schedule-solve"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        // Quit the pass if this attribute does not have the #[schedule] attribute
        if let Some(&goal) = data.comp.attrs.get(CompNum::Schedule) {
            self.goal = goal.into()
        } else {
            return Action::Stop;
        }

        // make sure this component only has one event
        assert_eq!(
            data.comp.events().idx_iter().count(),
            1,
            "Attempting to schedule a component with multiple events"
        );

        // For the inputs and outputs of the component, we need to schedule them as expected.
        for (pidx, port) in data.comp.inputs().chain(data.comp.outputs()) {
            let ir::Range { start, end } = port.live.range;

            let start = data.comp.get(start).offset.concrete(&data.comp);
            let end = data.comp.get(end).offset.concrete(&data.comp);

            let (start_expr, end_expr) = self.get_port_name(pidx);

            // Declare constants for the start and end of the port
            let start_expr = self
                .sol
                .declare_const(start_expr, self.sol.int_sort())
                .unwrap();
            let end_expr = self
                .sol
                .declare_const(end_expr, self.sol.int_sort())
                .unwrap();

            // These ports must occur exactly when they are scheduled
            self.sol
                .assert(self.sol.eq(self.sol.numeral(start), start_expr))
                .unwrap();

            self.sol
                .assert(self.sol.eq(self.sol.numeral(end), end_expr))
                .unwrap();
        }

        Action::Continue
    }

    fn connect(
        &mut self,
        con: &mut ir::Connect,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &data.comp;
        let ir::Connect { src, dst, .. } = con;

        // Make sure there are no bundles
        if !(src.is_port(comp)
            && dst.is_port(comp)
            && src.port.is_not_bundle(comp)
            && dst.port.is_not_bundle(comp))
        {
            unreachable!(
                "Port {} and {} are bundles. Bundles are not supported in the scheduling pass. Please run bundle-elim first.",
                comp.display(src.port),
                comp.display(dst.port)
            );
        }

        let (src_start, src_end) = self.get_port(src.port);
        let (dst_start, dst_end) = self.get_port(dst.port);

        let width = comp.get(src.port).width.concrete(comp);

        log::trace!(
            "Connecting {}: {} to {}: {}",
            src.port,
            self.sol.display(src_start),
            dst.port,
            self.sol.display(dst_start)
        );

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
        self.minimize_expr = self.sol.plus(self.minimize_expr, reg_expr);

        Action::Continue
    }

    fn invoke(
        &mut self,
        inv_idx: ir::InvIdx,
        data: &mut VisitorData,
    ) -> Action {
        let comp = &data.comp;

        log::trace!("Scheduling invocation {}", comp.display(inv_idx));

        let inv = comp.get(inv_idx);

        // Get the events of the invocation as variables
        let events: ir::SparseInfoMap<ir::Event, SExpr> = inv
            .events
            .iter()
            .map(|ir::EventBind { base, .. }| {
                let sexpr = self
                    .sol
                    .declare_const(
                        self.get_evt_name(inv_idx, *base),
                        self.sol.int_sort(),
                    )
                    .unwrap();

                // Make sure that the event is scheduled at a positive time
                self.sol
                    .assert(self.sol.gte(sexpr, self.sol.numeral(0)))
                    .unwrap();

                (base.key(), sexpr)
            })
            .collect();

        // Intern all the constraints for the events
        let foreign_comp = data.ctx().get(inv.inst.comp(comp));
        for &constraint in foreign_comp.get_event_asserts() {
            self.sol
                .assert(self.prop_to_sexp(foreign_comp, &events, constraint))
                .unwrap();
        }

        for pidx in inv.ports.iter() {
            let port = comp.get(*pidx);
            let ir::Port {
                owner:
                    ir::PortOwner::Inv {
                        base: foreign_pidx, ..
                    },
                ..
            } = port
            else {
                unreachable!("Port {} is not owned by an invocation", pidx)
            };

            let (start_expr, end_expr, start, end) = foreign_pidx.apply(
                |p, foreign_comp| {
                    let ir::Range { start, end } =
                        foreign_comp.get(p).live.range;

                    let start_expr = *events.get(foreign_comp.get(start).event);
                    let end_expr = *events.get(foreign_comp.get(end).event);

                    let start =
                        foreign_comp.get(start).offset.concrete(foreign_comp);
                    let end =
                        foreign_comp.get(end).offset.concrete(foreign_comp);

                    (start_expr, end_expr, start, end)
                },
                data.ctx(),
            );

            // Create expressions for the ports relative to the invocation
            let start_expr = self.sol.plus(start_expr, self.sol.numeral(start));

            let end_expr = self.sol.plus(end_expr, self.sol.numeral(end));

            log::trace!(
                "Port {} is live from {} to {}",
                pidx,
                self.sol.display(start_expr),
                self.sol.display(end_expr)
            );

            // Declare constants for the start and end of the port
            let (start_var, end_var) = self.get_port_name(*pidx);

            let start_var = self
                .sol
                .declare_const(start_var, self.sol.int_sort())
                .unwrap();
            let end_var = self
                .sol
                .declare_const(end_var, self.sol.int_sort())
                .unwrap();

            // These ports must occur exactly when they are scheduled
            self.sol.assert(self.sol.eq(start_expr, start_var)).unwrap();
            self.sol.assert(self.sol.eq(end_expr, end_var)).unwrap();
        }

        Action::Continue
    }

    fn end(&mut self, data: &mut VisitorData) {
        self.combinational_delays(&data.comp, data.ctx());

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

        let event = data.comp.events().idx_iter().next().unwrap();

        // Loop through invocations and find what they're bound to
        // collect here to let us mutate [data.comp] inside the loop
        for inv_idx in data.comp.invocations().idx_iter() {
            let new_times = data
                .comp
                .get(inv_idx)
                .events
                .iter()
                .map(|ir::EventBind { base, arg, .. }| {
                    let SExprData::Atom(s) =
                        self.sol.get(self.get_inv_evt(inv_idx, *base))
                    else {
                        unreachable!()
                    };

                    (arg.event(&data.comp), *bindings.get(s).unwrap())
                })
                .collect_vec();

            // Needds to be broken up because we need mutable access to data.comp
            let new_times = new_times
                .into_iter()
                .map(|(event, time)| {
                    let time = data.comp.add(ir::Expr::Concrete(time));

                    let time = data.comp.add(ir::Time {
                        event,
                        offset: time,
                    });

                    log::debug!(
                        "Invocation {} scheduled at cycle {}",
                        data.comp.display(inv_idx),
                        data.comp.display(time)
                    );
                    time
                })
                .collect_vec();

            for (ir::EventBind { arg, .. }, time) in
                data.comp.get_mut(inv_idx).events.iter_mut().zip(new_times)
            {
                *arg = time
            }
        }

        // Loop through ports and set their live ranges
        for pidx in data.comp.ports().idx_iter() {
            let (start, end) = self.get_port(pidx);

            let SExprData::Atom(s) = self.sol.get(start) else {
                unreachable!(
                    "Expected start of port {} to be an atom, got {}",
                    pidx,
                    self.sol.display(start)
                )
            };

            let start = *bindings.get(s).unwrap();

            let SExprData::Atom(s) = self.sol.get(end) else {
                unreachable!(
                    "Expected end of port {} to be an atom, got {}",
                    pidx,
                    self.sol.display(end)
                )
            };

            let end = *bindings.get(s).unwrap();

            let start = data.comp.add(ir::Expr::Concrete(start));
            let end = data.comp.add(ir::Expr::Concrete(end));

            let start = data.comp.add(ir::Time {
                event,
                offset: start,
            });

            let end = data.comp.add(ir::Time { event, offset: end });

            log::debug!(
                "Port {} scheduled to be live from [{}, {}]",
                data.comp.display(pidx),
                data.comp.display(start),
                data.comp.display(end)
            );

            let port = data.comp.get_mut(pidx);

            port.live.range = ir::Range { start, end };
        }
    }
}
