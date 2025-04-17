use fil_ir::{self as ir, AddCtx, Ctx, DisplayCtx, MutCtx};
use itertools::Itertools;
pub struct Concretize<'comp> {
    comp: &'comp mut ir::Component,
    binding: ir::Bind<ir::ParamIdx, u64>,

    expr_map: ir::SparseInfoMap<ir::Expr, ir::ExprIdx>,
    prop_map: ir::SparseInfoMap<ir::Prop, ir::PropIdx>,
    time_map: ir::SparseInfoMap<ir::Time, ir::TimeIdx>,
}

impl<'comp> Concretize<'comp> {
    pub fn new(comp: &'comp mut ir::Component) -> Self {
        Self {
            comp,
            binding: Default::default(),
            expr_map: Default::default(),
            prop_map: Default::default(),
            time_map: Default::default(),
        }
    }

    /// Take the binding map
    pub fn take(self) -> ir::Bind<ir::ParamIdx, u64> {
        self.binding
    }
}

impl Concretize<'_> {
    pub fn comp(&mut self) {
        // Monomorphize commands
        self.comp.cmds = self.commands(self.comp.cmds.clone());

        // Monomorphize the signature
        self.sig();
    }

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

    fn event(&mut self, event: ir::EventIdx) {
        let ir::Event { delay, .. } = self.comp.get(event).clone();

        let delay = self.timesub(delay);

        self.comp.get_mut(event).delay = delay;
    }

    fn port(&mut self, port: ir::PortIdx) {
        let ir::Port {
            width,
            live: ir::Liveness { lens, range, .. },
            ..
        } = self.comp.get(port).clone();

        let width = self.expr(width);

        let lens = lens.into_iter().map(|lens| self.expr(lens)).collect_vec();
        let range = ir::Range {
            start: self.time(range.start),
            end: self.time(range.end),
        };

        let p = self.comp.get_mut(port);

        p.width = width;
        p.live.lens = lens;
        p.live.range = range;
    }

    fn commands(&mut self, cmds: Vec<ir::Command>) -> Vec<ir::Command> {
        let mut new_cmds = Vec::new();
        for cmd in cmds {
            let cmds = match cmd {
                ir::Command::Instance(inst) => {
                    let ir::Instance { args, lives, .. } =
                        self.comp.get(inst).clone();
                    let args = args
                        .into_iter()
                        .map(|arg| self.expr(arg))
                        .collect_vec()
                        .into_boxed_slice();
                    let lives = lives
                        .into_iter()
                        .map(|ir::Range { start, end }| {
                            let start = self.time(start);
                            let end = self.time(end);
                            ir::Range { start, end }
                        })
                        .collect_vec();

                    let m = self.comp.get_mut(inst);

                    m.args = args;
                    m.lives = lives;

                    Some(inst.into())
                }
                ir::Command::Invoke(invoke) => {
                    let ir::Invoke { events, ports, .. } =
                        self.comp.get(invoke).clone();

                    for port in ports {
                        self.port(port);
                    }

                    let events = events
                        .into_iter()
                        .map(
                            |ir::EventBind {
                                 arg,
                                 info,
                                 base,
                                 delay,
                             }| {
                                ir::EventBind {
                                    arg: self.time(arg),
                                    delay: self.timesub(delay),
                                    info,
                                    base,
                                }
                            },
                        )
                        .collect_vec();

                    let n = self.comp.get_mut(invoke);

                    n.events = events;

                    Some(invoke.into())
                }
                ir::Command::BundleDef(port) => {
                    self.port(port);
                    Some(port.into())
                }
                ir::Command::Connect(ir::Connect {
                    mut src,
                    mut dst,
                    info,
                }) => {
                    src.ranges = src
                        .ranges
                        .into_iter()
                        .map(|(start, end)| {
                            let start = self.expr(start);
                            let end = self.expr(end);
                            (start, end)
                        })
                        .collect_vec();

                    dst.ranges = dst
                        .ranges
                        .into_iter()
                        .map(|(start, end)| {
                            let start = self.expr(start);
                            let end = self.expr(end);
                            (start, end)
                        })
                        .collect_vec();

                    Some(ir::Connect { src, dst, info }.into())
                }
                ir::Command::Let(ir::Let { param, expr }) => {
                    let ir::MaybeUnknown::Known(expr) = expr else {
                        unreachable!("Let bindings should already be bound")
                    };

                    let expr = self.expr(expr).concrete(self.comp);

                    log::trace!(
                        "Let {} bound to {}",
                        self.comp.display(param),
                        expr
                    );

                    self.binding.push(param, expr);

                    None
                }
                ir::Command::Exists(ir::Exists { param, expr }) => {
                    let v = self.expr(expr).concrete(self.comp);

                    log::trace!(
                        "Existential {} bound to {}",
                        self.comp.display(param),
                        v
                    );

                    self.binding.push(param, v);

                    None
                }
                ir::Command::Fact(ir::Fact { prop, reason, .. }) => {
                    // Make everything an assert after monomorphize
                    let prop = self.prop(prop);
                    self.comp.assert(prop, reason)
                }
                ir::Command::ForLoop(_) | fil_ir::Command::If(_) => {
                    unreachable!(
                        "Forloops and Ifs should have been monomorphized away."
                    )
                }
            };

            new_cmds.extend(cmds);
        }

        new_cmds
    }

    fn sig(&mut self) {
        // Monomorphize events
        for event in self.comp.events().idx_iter() {
            self.event(event);
        }

        // Monomorphize signature ports
        for port in self.comp.ports().idx_iter() {
            if self.comp.get(port).is_sig() {
                self.port(port);
            }
        }
    }
}
