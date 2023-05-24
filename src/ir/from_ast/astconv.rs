//! Convert the frontend AST to the IR.
use super::{BuildCtx, Sig, SigMap};
use crate::ir::{
    Cmp, CompIdx, Ctx, ExprIdx, ParamIdx, PortIdx, PropIdx, TimeIdx,
};
use crate::{
    ast::{self, Id},
    ir,
    utils::Binding,
};
use itertools::Itertools;
use std::{iter, rc::Rc};

impl<'ctx, 'prog> BuildCtx<'ctx, 'prog> {
    fn expr(&mut self, expr: ast::Expr) -> ExprIdx {
        match expr {
            ast::Expr::Abstract(p) => {
                let Some(&pidx) = self.param_map.get(&p) else {
                    unreachable!("Parameter {p} not found")
                };
                self.comp.add(ir::Expr::Param(pidx))
            }
            ast::Expr::Concrete(n) => {
                let e = ir::Expr::Concrete(n);
                self.comp.add(e)
            }
            ast::Expr::Op { op, left, right } => {
                let l = self.expr(*left);
                let r = self.expr(*right);
                match op {
                    ast::Op::Add => l.add(r, self.comp),
                    ast::Op::Mul => l.mul(r, self.comp),
                    ast::Op::Sub => l.sub(r, self.comp),
                    ast::Op::Div => l.div(r, self.comp),
                    ast::Op::Mod => l.rem(r, self.comp),
                }
            }
            ast::Expr::App { func, arg } => {
                let arg = self.expr(*arg);
                match func {
                    ast::UnFn::Pow2 => arg.pow2(self.comp),
                    ast::UnFn::Log2 => arg.log2(self.comp),
                }
            }
        }
    }

    fn expr_cons(&mut self, cons: ast::OrderConstraint<ast::Expr>) -> PropIdx {
        let lhs = self.expr(cons.left);
        let rhs = self.expr(cons.right);
        let op = match cons.op {
            ast::OrderOp::Gt => Cmp::Gt,
            ast::OrderOp::Gte => Cmp::Gte,
            ast::OrderOp::Eq => Cmp::Eq,
        };
        self.comp.add(ir::Prop::Cmp { lhs, op, rhs })
    }

    fn implication(&mut self, i: ast::Implication<ast::Expr>) -> PropIdx {
        let cons = self.expr_cons(i.cons);
        if let Some(ante) = i.guard {
            let ante = self.expr_cons(ante);
            ante.implies(cons, self.comp)
        } else {
            cons
        }
    }

    /// Add a parameter to the component.
    fn param(&mut self, param: ast::Id) -> ParamIdx {
        // TODO(rachit): Parameters do not have default values yet.
        let idx = self.comp.add(ir::Param::default());
        self.param_map.insert(param, idx);
        idx
    }

    fn time(&mut self, t: ast::Time) -> TimeIdx {
        let Some(event) = self.event_map.get(&t.event).copied() else {
            unreachable!("Event {} not found. Map:\n{}", t.event, self.event_map)
        };
        let offset = self.expr(t.offset);
        self.comp.add(ir::Time { event, offset })
    }

    fn timesub(&mut self, ts: ast::TimeSub) -> ir::TimeSub {
        match ts {
            ast::TimeSub::Unit(e) => ir::TimeSub::Unit(self.expr(e)),
            ast::TimeSub::Sym { l, r } => {
                let l = self.time(l);
                let r = self.time(r);
                l.sub(r, self.comp)
            }
        }
    }

    /// Add an event to the component.
    fn event(&mut self, eb: ast::EventBind) {
        let delay = self.timesub(eb.delay.take());
        let default = eb.default.map(|t| self.time(t));
        let e = ir::Event { delay, default };
        let idx = self.comp.add(e);
        self.event_map.insert(*eb.event, idx);
    }

    fn range(&mut self, r: ast::Range) -> ir::Range {
        let start = self.time(r.start);
        let end = self.time(r.end);
        ir::Range { start, end }
    }

    fn port(&mut self, pd: ast::PortDef, owner: ir::PortOwner) -> PortIdx {
        let (name, port, owner) = match pd {
            ast::PortDef::Port {
                name,
                liveness,
                bitwidth,
            } => {
                // The bundle type uses a fake bundle index and has a length of 1.
                // We don't need to push a new scope because this type is does not
                // bind any new parameters.
                let live = self.with_scope(|ctx| ir::Liveness {
                    idx: ctx.param(Id::default()), // This parameter is unused
                    len: ctx.comp.num(1),
                    range: ctx.range(liveness.take()),
                });

                let p = ir::Port {
                    width: self.expr(bitwidth.take()),
                    owner: owner.clone(),
                    live,
                };
                (name, p, owner)
            }
            ast::PortDef::Bundle(ast::Bundle {
                name,
                typ:
                    ast::BundleType {
                        idx,
                        len,
                        liveness,
                        bitwidth,
                    },
            }) => {
                // Construct the bundle type in a new scope.
                let live = self.with_scope(|ctx| ir::Liveness {
                    idx: ctx.param(*idx),
                    len: ctx.expr(len.take()),
                    range: ctx.range(liveness.take()),
                });

                let p = ir::Port {
                    width: self.expr(bitwidth.take()),
                    owner: owner.clone(),
                    live,
                };
                (name, p, owner)
            }
        };
        let idx = self.comp.add(port);
        self.add_port(*name, owner, idx);
        idx
    }

    /// Transforms an access into (start, end)
    fn access(&mut self, access: ast::Access) -> (ir::ExprIdx, ir::ExprIdx) {
        match access {
            ast::Access::Index(n) => {
                let n = self.expr(n);
                (n, n.add(self.comp.num(1), self.comp))
            }
            ast::Access::Range { start, end } => {
                (self.expr(start), self.expr(end))
            }
        }
    }

    /// Get the index associated with an AST port. The port must have been
    /// previously defined.
    fn get_access(
        &mut self,
        port: ast::Port,
        dir: ir::Direction,
    ) -> ir::Access {
        match port {
            ast::Port::This(n) => {
                let owner = ir::PortOwner::Sig { dir };
                ir::Access::port(self.get_port(n.copy(), owner), self.comp)
            }
            ast::Port::InvPort { invoke, name } => {
                let inv = *self.inv_map.get(&invoke.copy()).unwrap();
                let owner = ir::PortOwner::Inv { inv, dir };
                ir::Access::port(self.get_port(name.copy(), owner), self.comp)
            }
            ast::Port::Bundle { name, access } => {
                // NOTE(rachit): The AST does not distinguish between bundles
                // defined by the signature and locally defined bundles so we
                // must search both.
                let owner = ir::PortOwner::Sig { dir };
                let port = if let Some(p) = self.find_port(name.copy(), owner) {
                    p
                } else {
                    let owner = ir::PortOwner::Local;
                    self.get_port(name.copy(), owner)
                };
                let (start, end) = self.access(access.take());
                ir::Access { port, start, end }
            }
            ast::Port::InvBundle {
                invoke,
                port,
                access,
            } => {
                let inv = *self.inv_map.get(&invoke.copy()).unwrap();
                let owner = ir::PortOwner::Inv { inv, dir };
                let port = self.get_port(port.copy(), owner);
                let (start, end) = self.access(access.take());
                ir::Access { port, start, end }
            }
            ast::Port::Constant(_) => todo!("Constant ports"),
        }
    }

    fn sig(&mut self, sig: ast::Signature) {
        for param in &sig.params {
            self.param(param.copy());
        }
        for event in &sig.events {
            // XXX(rachit): Unnecessary clone.
            self.event(event.clone().take());
        }
        for port in sig.inputs() {
            // XXX(rachit): Unnecessary clone.
            self.port(port.inner().clone(), ir::PortOwner::sig_out());
        }
        for port in sig.outputs() {
            // XXX(rachit): Unnecessary clone.
            self.port(port.inner().clone(), ir::PortOwner::sig_in());
        }
    }

    fn instance(&mut self, inst: ast::Instance) -> Vec<ir::Command> {
        let ast::Instance {
            name,
            component,
            bindings,
        } = inst;
        // Add the facts defined by the instance as assertions in the
        // component.
        let comp = self.sigs.get(&component).unwrap().clone();
        let binding = Binding::new(
            comp.params
                .iter()
                .copied()
                .zip(bindings.clone().into_iter().map(|b| b.take())),
        );
        let facts = comp
            .facts
            .into_iter()
            .map(|f| {
                let p = f.resolve_expr(&binding);
                let prop = self.expr_cons(p);
                // This is a checked fact because the calling component needs to
                // honor it.
                ir::Fact::assert(prop).into()
            })
            .collect_vec();
        let inst = ir::Instance {
            comp: comp.idx,
            params: bindings
                .into_iter()
                .map(|b| self.expr(b.take()))
                .collect_vec()
                .into_boxed_slice(),
        };
        let idx = self.comp.add(inst);
        self.inst_map.insert(name.copy(), idx);
        // Track the component binding for this instance
        self.inst_to_sig.push(idx, (Rc::new(binding), *component));
        iter::once(ir::Command::from(idx))
            .chain(facts.into_iter())
            .collect_vec()
    }

    /// Invokes are the most complicated construct to compile. This function:
    /// 1. Creates a new invoke in the component with the time bindings.
    /// 2. Resolves output ports and defines them in the component
    /// 3. Generates connections implied by the arguments to the invoke.
    fn invoke(&mut self, inv: ast::Invoke) -> Vec<ir::Command> {
        let ast::Invoke {
            name,
            instance,
            abstract_vars,
            ports,
        } = inv;
        let inst = *self.inst_map.get(&instance).unwrap();
        let events = abstract_vars
            .iter()
            .map(|v| self.time(v.clone().take()))
            .collect_vec()
            .into_boxed_slice();
        let inv = self.comp.add(ir::Invoke { inst, events });
        self.inv_map.insert(name.copy(), inv);

        let Some(ports) = ports else {
            unreachable!("Low-level invokes not supported")
        };

        // The inputs
        let srcs = ports
            .into_iter()
            .map(|p| self.get_access(p.take(), ir::Direction::Out))
            .collect_vec();
        let (param_binding, comp) = self.inst_to_sig.get(inst).clone();
        let sig = self.sigs.get(&comp).unwrap();

        // Event bindings
        let event_binding = Binding::new(
            sig.events
                .iter()
                .zip(abstract_vars.iter())
                .map(|(e, t)| (*e, t.clone().take())),
        );

        // Define the output port from the invoke
        for p in sig.outputs.clone() {
            let resolved = p
                .resolve_exprs(&param_binding)
                .resolve_event(&event_binding);
            let owner = ir::PortOwner::Inv {
                inv,
                dir: ir::Direction::Out,
            };
            self.port(resolved, owner);
        }

        assert!(
            sig.inputs.len() == srcs.len(),
            "signature defined {} inputs but provided {} arguments",
            sig.inputs.len(),
            srcs.len()
        );
        let connects =
            sig.inputs.clone().into_iter().zip(srcs).map(|(p, src)| {
                let resolved = p
                    .resolve_exprs(&param_binding)
                    .resolve_event(&event_binding);
                let owner = ir::PortOwner::Inv {
                    inv,
                    dir: ir::Direction::In,
                };
                // Add the port to the component
                let pidx = self.port(resolved, owner);
                let end = self.comp[pidx].live.len;
                let dst = ir::Access {
                    port: pidx,
                    start: self.comp.num(0),
                    end,
                };
                ir::Connect { src, dst }.into()
            });

        iter::once(ir::Command::from(inv))
            .chain(connects)
            .collect_vec()
    }

    fn commands(&mut self, cmds: Vec<ast::Command>) -> Vec<ir::Command> {
        cmds.into_iter().flat_map(|c| self.command(c)).collect_vec()
    }

    fn command(&mut self, cmd: ast::Command) -> Vec<ir::Command> {
        match cmd {
            ast::Command::Invoke(inv) => self.invoke(inv),
            ast::Command::Instance(inst) => self.instance(inst),
            ast::Command::Fact(ast::Fact { cons, checked }) => {
                let prop = self.implication(cons.take());
                let fact = if checked {
                    ir::Fact::assert(prop)
                } else {
                    ir::Fact::assume(prop)
                };
                vec![fact.into()]
            }
            ast::Command::Connect(ast::Connect { src, dst, guard }) => {
                assert!(guard.is_none(), "Guards are not supported");
                let src = self.get_access(src.take(), ir::Direction::Out);
                let dst = self.get_access(dst.take(), ir::Direction::In);
                vec![ir::Connect { src, dst }.into()]
            }
            ast::Command::ForLoop(ast::ForLoop {
                idx,
                start,
                end,
                body,
            }) => {
                let start = self.expr(start);
                let end = self.expr(end);
                // Compile the body in a new scope
                let (index, body) = self.with_scope(|this| {
                    let idx = this.param(idx.take());
                    (idx, this.commands(body))
                });
                let l = ir::Loop {
                    index,
                    start,
                    end,
                    body,
                };
                vec![l.into()]
            }
            ast::Command::If(ast::If { cond, then, alt }) => {
                let cond = self.expr_cons(cond);
                let then = self.commands(then);
                let alt = self.commands(alt);
                vec![ir::If { cond, then, alt }.into()]
            }
            ast::Command::Bundle(bun) => {
                // Add the bundle to the current scope
                self.port(ast::PortDef::Bundle(bun), ir::PortOwner::Local);
                vec![]
            }
        }
    }

    fn comp(
        comp: ast::Component,
        idx: CompIdx,
        sigs: &'prog SigMap,
    ) -> ir::Component {
        let mut ir_comp = ir::Component::new(idx);
        let mut builder = BuildCtx::new(&mut ir_comp, sigs);
        builder.sig(comp.sig);
        let cmds = builder.commands(comp.body);
        ir_comp.cmds = cmds;
        ir_comp
    }
}

pub fn transform(ns: ast::Namespace) -> ir::Context {
    let mut sig_map = SigMap::default();
    // Walk over sigs and build a SigMap
    for (idx, (_, sig)) in ns.signatures().enumerate() {
        sig_map.insert(sig.name.copy(), Sig::from((sig, idx)));
    }

    let mut ctx = ir::Context::default();
    for comp in ns.components {
        let idx = sig_map.get(&comp.sig.name).unwrap().idx;
        let ir_comp = BuildCtx::comp(comp, idx, &sig_map);
        ctx.comps.push(ir_comp);
    }
    ctx
}
