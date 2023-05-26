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

    fn event_cons(&mut self, cons: ast::OrderConstraint<ast::Time>) -> PropIdx {
        let lhs = self.time(cons.left);
        let rhs = self.time(cons.right);
        let op = match cons.op {
            ast::OrderOp::Gt => Cmp::Gt,
            ast::OrderOp::Gte => Cmp::Gte,
            ast::OrderOp::Eq => Cmp::Eq,
        };
        self.comp.add(ir::Prop::TimeCmp { lhs, op, rhs })
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
    fn param(&mut self, param: ast::Id, owner: ir::ParamOwner) -> ParamIdx {
        // TODO(rachit): Parameters do not have default values yet.
        let idx = self.comp.add(ir::Param::new(owner));
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
                    idx: ctx.param(Id::default(), ir::ParamOwner::Bundle), // This parameter is unused
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
                    idx: ctx.param(*idx, ir::ParamOwner::Bundle),
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
                let inv = self.get_inv(invoke.copy());
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
                let inv = self.get_inv(invoke.copy());
                let owner = ir::PortOwner::Inv { inv, dir };
                let port = self.get_port(port.copy(), owner);
                let (start, end) = self.access(access.take());
                ir::Access { port, start, end }
            }
            ast::Port::Constant(_) => todo!("Constant ports"),
        }
    }

    fn sig(&mut self, sig: ast::Signature) -> Vec<ir::Command> {
        for param in &sig.params {
            self.param(param.copy(), ir::ParamOwner::Sig);
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

        // Constraints defined by the signature
        let mut cons = Vec::with_capacity(
            sig.param_constraints.len() + sig.event_constraints.len(),
        );
        for ec in sig.event_constraints {
            cons.push(ir::Fact::assume(self.event_cons(ec.take())).into());
        }
        for pc in sig.param_constraints {
            cons.push(ir::Fact::assume(self.expr_cons(pc.take())).into());
        }

        cons
    }

    fn declare_inst(&mut self, inst: &ast::Instance) {
        let ast::Instance {
            name,
            component,
            bindings,
        } = inst;
        let comp = self.sigs.get(component).unwrap();
        let binding = Binding::new(
            comp.params
                .iter()
                .copied()
                .zip(bindings.clone().into_iter().map(|b| b.take())),
        );
        let inst = ir::Instance {
            comp: comp.idx,
            params: bindings
                .iter()
                .map(|b| self.expr(b.clone().take()))
                .collect_vec()
                .into_boxed_slice(),
        };
        let idx = self.comp.add(inst);
        self.inst_map.insert(name.copy(), idx);
        // Track the component binding for this instance
        self.inst_to_sig.push(idx, (Rc::new(binding), **component));
    }

    fn instance(&mut self, inst: ast::Instance) -> Vec<ir::Command> {
        // Add the facts defined by the instance as assertions in the
        // component.
        let idx = *self.inst_map.get(&inst.name).unwrap();
        let (binding, component) = self.inst_to_sig.get(idx).clone();
        let facts = self
            .sigs
            .get(&component)
            .unwrap()
            .param_cons
            .clone()
            .into_iter()
            .map(|f| {
                let p = f.resolve_expr(&binding);
                let prop = self.expr_cons(p);
                // This is a checked fact because the calling component needs to
                // honor it.
                ir::Fact::assert(prop).into()
            })
            .collect_vec();
        iter::once(ir::Command::from(idx))
            .chain(facts.into_iter())
            .collect_vec()
    }

    /// Construct an event binding from this Signature's events and the given
    /// arguments.
    /// Fills in the missing arguments with default values
    pub fn event_binding(
        &self,
        events: &[ast::EventBind],
        args: impl IntoIterator<Item = ast::Time>,
    ) -> Binding<ast::Time> {
        let args = args.into_iter().collect_vec();
        assert!(
            events.iter().take_while(|ev| ev.default.is_none()).count()
                <= args.len(),
            "Insuffient events for component invocation.",
        );

        let mut partial_map = Binding::new(
            events
                .iter()
                .map(|eb| eb.event.inner())
                .cloned()
                .zip(args.iter().cloned()),
        );
        // Skip the events that have been bound
        let remaining = events
            .iter()
            .skip(args.len())
            .map(|eb| {
                let bind = eb
                    .default
                    .as_ref()
                    .unwrap()
                    .clone()
                    .resolve_event(&partial_map);
                (*eb.event.inner(), bind)
            })
            .collect();

        partial_map.extend(remaining);
        partial_map
    }

    /// This function is called during the second pass of the conversion and
    /// generates the connections implied by the arguments to the invoke.
    fn invoke(&mut self, inv: ast::Invoke) -> Vec<ir::Command> {
        let ast::Invoke {
            name,
            abstract_vars,
            ports,
            ..
        } = inv;
        let Some(ports) = ports else {
            unreachable!("No ports provided for invocation {}", name)
        };
        let inv = self.get_inv(name.copy());
        let inst = self.comp[inv].inst;
        let (param_binding, comp) = self.inst_to_sig.get(inst).clone();
        let sig = self.sigs.get(&comp).unwrap();

        // Event bindings
        let event_binding = self.event_binding(
            &sig.events,
            abstract_vars.iter().map(|v| v.inner().clone()),
        );

        let srcs = ports
            .into_iter()
            .map(|p| self.get_access(p.take(), ir::Direction::Out))
            .collect_vec();
        assert!(
            sig.inputs.len() == srcs.len(),
            "signature defined {} inputs but provided {} arguments",
            sig.inputs.len(),
            srcs.len()
        );

        sig.inputs
            .clone()
            .into_iter()
            .zip(srcs)
            .map(|(p, src)| {
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
            })
            .chain(Some(ir::Command::from(inv)))
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
                    let idx = this.param(idx.take(), ir::ParamOwner::Local);
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

    /// Invokes are the most complicated construct to compile. This function:
    /// 1. Creates a new invoke in the component with the time bindings.
    /// 2. Resolves output ports and defines them in the component
    fn declare_inv(&mut self, inv: &ast::Invoke) {
        let ast::Invoke {
            name,
            instance,
            abstract_vars,
            ..
        } = inv;

        let inst = *self.inst_map.get(instance).unwrap();
        let events = abstract_vars
            .iter()
            .map(|v| self.time(v.clone().take()))
            .collect_vec()
            .into_boxed_slice();
        let inv = self.comp.add(ir::Invoke {
            inst,
            events,
            ports: vec![], // Filled in later
        });
        self.add_inv(name.copy(), inv);

        let mut def_ports = vec![];

        // The inputs
        let (param_binding, comp) = self.inst_to_sig.get(inst).clone();
        let sig = self.sigs.get(&comp).unwrap();

        // Event bindings
        let event_binding = self.event_binding(
            &sig.events,
            abstract_vars.iter().map(|v| v.inner().clone()),
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
            def_ports.push(self.port(resolved, owner));
        }

        // Add the inputs from the invoke. The outputs are added in the second
        // pass using [Self::add_invoke_connects].
        self.comp.invocations.get_mut(inv).ports = def_ports;
    }

    /// Walk over the component and declare all instances, invocations, and all outputs defined by invocations.
    /// This is needed because invocations are not required to be declared before they are used.
    fn declare_cmd(&mut self, cmd: &ast::Command) {
        match cmd {
            ast::Command::Instance(inst) => {
                self.declare_inst(inst);
            }
            ast::Command::Invoke(inv) => {
                self.declare_inv(inv);
            }
            ast::Command::ForLoop(ast::ForLoop { idx, body, .. }) => {
                self.param(idx.copy(), ir::ParamOwner::Local);
                self.declare_cmds(body);
            }
            ast::Command::If(ast::If { then, alt, .. }) => {
                self.declare_cmds(then);
                self.declare_cmds(alt);
            }
            ast::Command::Fact(_)
            | ast::Command::Connect(_)
            | ast::Command::Bundle(_) => { /* Handled in second pass */ }
        }
    }

    fn declare_cmds(&mut self, cmds: &[ast::Command]) {
        for cmd in cmds {
            self.declare_cmd(cmd);
        }
    }

    fn external(idx: CompIdx, sig: ast::Signature) -> ir::External {
        ir::External { idx, sig }
    }

    fn comp(
        comp: ast::Component,
        idx: CompIdx,
        sigs: &'prog SigMap,
    ) -> ir::Component {
        let mut ir_comp = ir::Component::new(idx);
        let mut builder = BuildCtx::new(&mut ir_comp, sigs);
        let mut cmds = builder.sig(comp.sig);
        builder.declare_cmds(&comp.body);
        let body_cmds = builder.commands(comp.body);
        cmds.extend(body_cmds);
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
    for (_, exts) in ns.externs {
        for ext in exts {
            let idx = sig_map.get(&ext.name).unwrap().idx;
            let ir_ext = BuildCtx::external(idx, ext);
            ctx.comps.checked_add(idx, ir::CompOrExt::Ext(ir_ext));
        }
    }

    for comp in ns.components {
        let idx = sig_map.get(&comp.sig.name).unwrap().idx;
        let ir_comp = BuildCtx::comp(comp, idx, &sig_map);
        ctx.comps.checked_add(idx, ir::CompOrExt::Comp(ir_comp));
    }
    ctx
}
