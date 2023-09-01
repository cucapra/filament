//! Convert the frontend AST to the IR.
use super::build_ctx::InvPort;
use super::{BuildCtx, Sig, SigMap};
use crate::diagnostics;
use crate::ir::{
    AddCtx, Cmp, Ctx, EventIdx, ExprIdx, InterfaceSrc, MutCtx, ParamIdx,
    PortIdx, PropIdx, TimeIdx,
};
use crate::utils::{GPosIdx, Idx};
use crate::{ast, ir};
use itertools::Itertools;
use std::collections::HashMap;
use std::{iter, rc::Rc};

pub type BuildRes<T> = Result<T, diagnostics::Diagnostics>;

/// # Declare phase
/// This is the first pass over a particular scope and responsible for forward declaring names defined by invocations.
/// We do this because invocation ports can be used before their definition:
/// ```
/// p = new Prev[32]<G>(add.out);
/// add = new Add[32]<G>(p.out, 1);
/// ```
///
/// At a high-level, it does the following:
/// * Define all the locally-bound parameters, specifically those defined by loops.
///   This is needed to correctly resolve the instances which may use parameters.
/// * For each instance in the program, compute the partially resolved component
///   signature which substitutes all parameters in the signature.
/// * For each invocation, compute the fully resolved signature (where events are correctly substituted)
///   and define all the parameters.
impl<'prog> BuildCtx<'prog> {
    fn declare_inst(&mut self, inst: &ast::Instance) -> BuildRes<()> {
        let ast::Instance {
            name,
            component,
            bindings,
        } = inst;

        let comp = self.get_sig(component)?;
        let mut binding = comp.param_binding(
            bindings.iter().map(|e| e.inner()).cloned().collect_vec(),
            component.clone(),
            self.diag(),
        )?;
        let inst = ir::Instance {
            comp: comp.idx,
            params: binding
                .iter()
                .map(|(_, b)| self.expr(b.clone()))
                .collect::<BuildRes<Vec<_>>>()?
                .into_boxed_slice(),
            info: self.comp().add(ir::Info::instance(
                name.copy(),
                component.pos(),
                name.pos(),
            )),
        };

        // Extend the binding with the let-bound parameters in the signature
        // Bindings can mention previous bindings so we add bindings as we
        // go along.
        comp.sig_binding.iter().for_each(
            |ast::ParamBind { param, default }| {
                let e = default.clone().unwrap().resolve(&binding);
                binding.extend([(param.copy(), e)])
            },
        );

        //println!("ir inst has bindings {:?}", inst.params);
        let idx = self.comp().add(inst);
        self.add_inst(name.copy(), idx);
        // Track the component binding for this instance
        self.inst_to_sig
            .push(idx, (Rc::new(binding), component.clone()));
        Ok(())
    }

    /// Invokes are the most complicated construct to compile. This function:
    /// 1. Creates a new invoke in the component with the time bindings.
    /// 2. Resolves output ports and defines them in the component
    fn declare_inv(&mut self, inv: &ast::Invoke) -> BuildRes<()> {
        let ast::Invoke {
            name,
            instance,
            abstract_vars,
            ..
        } = inv;
        let inst = self.get_inst(instance)?;
        let info = self.comp().add(ir::Info::invoke(
            name.copy(),
            instance.pos(),
            name.pos(),
        ));
        let inv = self.comp().add(ir::Invoke {
            inst,
            ports: vec![],  // Filled in later
            events: vec![], // Filled in later
            info,
        });
        // foreign component being invoked
        let foreign_comp = inv.comp(self.comp());
        self.add_inv(name.copy(), inv);

        let mut def_ports = vec![];

        // The inputs
        let (param_binding, comp) = self.inst_to_sig.get(inst).clone();
        let sig = self.get_sig(&comp)?;

        // Event bindings
        let event_binding = sig.event_binding(
            abstract_vars.iter().map(|v| v.inner().clone()),
            instance,
            self.diag(),
        )?;

        // Define the output port from the invoke
        for (idx, p) in sig.raw_outputs.clone().into_iter().enumerate() {
            let resolved = p
                .resolve_exprs(&param_binding)
                .resolve_event(&event_binding);

            let base = ir::Foreign::new(
                self.sig_from_idx(foreign_comp).outputs[idx],
                foreign_comp,
            );

            let owner = ir::PortOwner::Inv {
                inv,
                dir: ir::Direction::Out,
                base,
            };
            def_ports.push(self.port(resolved, owner)?);
        }

        // Add the inputs from the invoke. The outputs are added in the second
        // pass over the AST.
        self.comp().get_mut(inv).ports.extend(def_ports);
        Ok(())
    }

    /// Declare the instances and invokes in the current scope.
    /// This does not burrow into inner scopes.
    fn declare_cmd(&mut self, cmd: &ast::Command) -> BuildRes<()> {
        match cmd {
            ast::Command::Instance(inst) => self.declare_inst(inst),
            ast::Command::Invoke(inv) => self.declare_inv(inv),
            ast::Command::ParamLet(ast::ParamLet { name, expr }) => {
                // Declare the parameter since it may be used in instance or
                // invocation definitions.
                let expr = self.expr(expr.clone())?;
                self.add_let_param(name.copy(), expr);
                Ok(())
            }
            ast::Command::ForLoop(_)
            | ast::Command::If(_)
            | ast::Command::Fact(_)
            | ast::Command::Connect(_)
            | ast::Command::Bundle(_) => Ok(()),
        }
    }

    fn declare_cmds(&mut self, cmds: &[ast::Command]) -> BuildRes<()> {
        for cmd in cmds {
            self.declare_cmd(cmd)?
        }
        Ok(())
    }
}

impl<'prog> BuildCtx<'prog> {
    fn expr(&mut self, expr: ast::Expr) -> BuildRes<ExprIdx> {
        let expr = match expr {
            ast::Expr::Abstract(p) => self.get_param(&p)?,
            ast::Expr::Concrete(n) => {
                let e = ir::Expr::Concrete(n);
                self.comp().add(e)
            }
            ast::Expr::Op { op, left, right } => {
                let lhs = self.expr(*left)?;
                let rhs = self.expr(*right)?;
                // The .add call simplifies the expression if possible
                self.comp().add(ir::Expr::Bin { op, lhs, rhs })
            }
            ast::Expr::App { func, arg } => {
                let arg = self.expr(*arg)?;
                // The .add call simplifies the expression if possible
                self.comp().add(ir::Expr::Fn {
                    op: func,
                    args: vec![arg],
                })
            }
        };
        Ok(expr)
    }

    fn expr_cons(
        &mut self,
        cons: ast::OrderConstraint<ast::Expr>,
    ) -> BuildRes<PropIdx> {
        let lhs = self.expr(cons.left)?;
        let rhs = self.expr(cons.right)?;
        let op = match cons.op {
            ast::OrderOp::Gt => Cmp::Gt,
            ast::OrderOp::Gte => Cmp::Gte,
            ast::OrderOp::Eq => Cmp::Eq,
        };
        Ok(self.comp().add(ir::Prop::Cmp(ir::CmpOp { lhs, op, rhs })))
    }

    fn event_cons(
        &mut self,
        cons: ast::OrderConstraint<ast::Time>,
    ) -> BuildRes<PropIdx> {
        let lhs = self.time(cons.left)?;
        let rhs = self.time(cons.right)?;
        let op = match cons.op {
            ast::OrderOp::Gt => Cmp::Gt,
            ast::OrderOp::Gte => Cmp::Gte,
            ast::OrderOp::Eq => Cmp::Eq,
        };
        Ok(self
            .comp()
            .add(ir::Prop::TimeCmp(ir::CmpOp { lhs, op, rhs })))
    }

    fn implication(
        &mut self,
        i: ast::Implication<ast::Expr>,
    ) -> BuildRes<PropIdx> {
        let cons = self.expr_cons(i.cons)?;
        if let Some(ante) = i.guard {
            let ante = self.expr_cons(ante)?;
            Ok(ante.implies(cons, self.comp()))
        } else {
            Ok(cons)
        }
    }

    /// Add a parameter to the component.
    fn param(
        &mut self,
        param: &ast::ParamBind,
        owner: ir::ParamOwner,
    ) -> ParamIdx {
        let info = self.comp().add(ir::Info::param(param.name(), param.pos()));
        let ir_param = ir::Param::new(owner, info);
        let is_sig_owned = ir_param.is_sig_owned();
        let idx = self.comp().add(ir_param);
        self.add_param(param.name(), idx);

        // only add information if this is a signature defined parameter
        if is_sig_owned {
            // If the component is expecting interface information, add it.
            if let Some(src) = &mut self.comp().src_info {
                src.params.insert(idx, param.name());
            }
        }

        idx
    }

    fn time(&mut self, t: ast::Time) -> BuildRes<TimeIdx> {
        let event = self.get_event(&t.event)?;
        let offset = self.expr(t.offset)?;
        Ok(self.comp().add(ir::Time { event, offset }))
    }

    fn timesub(&mut self, ts: ast::TimeSub) -> BuildRes<ir::TimeSub> {
        let ts = match ts {
            ast::TimeSub::Unit(e) => ir::TimeSub::Unit(self.expr(e)?),
            ast::TimeSub::Sym { l, r } => {
                let l = self.time(l)?;
                let r = self.time(r)?;
                l.sub(r, self.comp())
            }
        };
        Ok(ts)
    }

    /// Forward declare an event without adding its delay. We need to do this
    /// since delays of events may mention the event itself.
    /// `interface_port` is the optional interface port associated with this event.
    fn declare_event(
        &mut self,
        eb: &ast::EventBind,
        interface_port: Option<(ast::Id, GPosIdx)>,
    ) -> EventIdx {
        let info = self.comp().add(ir::Info::event(
            eb.event.copy(),
            eb.event.pos(),
            eb.delay.pos(),
            interface_port,
        ));
        // Add a fake delay of 0.
        let e = ir::Event {
            delay: self.comp().num(0).into(),
            info,
            has_interface: interface_port.is_some(),
        };
        let idx = self.comp().add(e);

        // If the component is expecting interface information, add it
        if let Some(src) = &mut self.comp().src_info {
            // add interface port if exists
            if let Some((name, _)) = interface_port {
                src.interface_ports.insert(idx, name);
            }
            // add event name (used by dump_interface)
            src.events.insert(idx, *eb.event);
        }

        log::trace!("Added event {} as {idx}", eb.event);
        self.add_event(*eb.event, idx);
        idx
    }

    fn range(&mut self, r: ast::Range) -> BuildRes<ir::Range> {
        let start = self.time(r.start)?;
        let end = self.time(r.end)?;
        Ok(ir::Range { start, end })
    }

    fn port(
        &mut self,
        pd: ast::PortDef,
        owner: ir::PortOwner,
    ) -> BuildRes<PortIdx> {
        let (name, p) = match pd {
            ast::PortDef::Port {
                name,
                liveness,
                bitwidth,
            } => {
                let info = self.comp().add(ir::Info::port(
                    name.copy(),
                    name.pos(),
                    bitwidth.pos(),
                    liveness.pos(),
                ));

                // The bundle type uses a fake bundle index and has a length of 1.
                // We don't need to push a new scope because this type is does not
                // bind any new parameters.
                let p_name = self.gen_name();
                let live = self.try_with_scope(|ctx| {
                    Ok(ir::Liveness {
                        idx: ctx.param(
                            &ast::ParamBind::from(p_name),
                            // Updated after the port is constructed
                            ir::ParamOwner::bundle(ir::PortIdx::UNKNOWN),
                        ), // This parameter is unused
                        len: ctx.comp().num(1),
                        range: ctx.range(liveness.take())?,
                    })
                })?;
                let p = ir::Port {
                    width: self.expr(bitwidth.take())?,
                    owner,
                    live,
                    info,
                };
                (name, p)
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
                let info = self.comp().add(ir::Info::port(
                    name.copy(),
                    name.pos(),
                    bitwidth.pos(),
                    liveness.pos(),
                ));
                // Construct the bundle type in a new scope.
                let live = self.try_with_scope(|ctx| {
                    Ok(ir::Liveness {
                        idx: ctx.param(
                            // Updated after the port is constructed
                            &ast::ParamBind::from(idx),
                            ir::ParamOwner::bundle(PortIdx::UNKNOWN),
                        ),
                        len: ctx.expr(len.take())?,
                        range: ctx.range(liveness.take())?,
                    })
                })?;
                let p = ir::Port {
                    width: self.expr(bitwidth.take())?,
                    owner,
                    live,
                    info,
                };
                (name, p)
            }
        };

        // Defines helper variable here due to lifetime issues
        let is_sig_port = p.is_sig();
        let idx = self.comp().add(p);
        // Fixup the liveness index parameter's owner
        let p = self.comp().get(idx).live.idx;
        let param = self.comp().get_mut(p);
        param.owner = ir::ParamOwner::bundle(idx);

        // If this is a signature port, try adding it to the component's external interface
        if is_sig_port {
            // If the component is expecting interface information, add it.
            if let Some(src) = &mut self.comp().src_info {
                src.ports.insert(idx, name.copy());
            }
        }

        // Add the port to the current scope
        self.add_port(name, idx);

        Ok(idx)
    }

    /// Transforms an access into (start, end)
    fn access(
        &mut self,
        access: ast::Access,
    ) -> BuildRes<(ir::ExprIdx, ir::ExprIdx)> {
        Ok((self.expr(access.start)?, self.expr(access.end)?))
    }

    /// Get the index associated with an AST port. The port must have been
    /// previously defined.
    fn get_access(
        &mut self,
        port: ast::Port,
        dir: ir::Direction,
    ) -> BuildRes<ir::Access> {
        let acc = match port {
            ast::Port::This(n) => {
                // NOTE: The AST does not distinguish between ports
                // defined by the signature and locally defined ports so we
                // must search both.
                let owner = InvPort::Sig(dir, n.clone());
                let port = if let Some(port) = self.find_port(&owner) {
                    port
                } else {
                    let owner = InvPort::Local(n);
                    self.get_port(&owner)?
                };

                ir::Access::port(port, self.comp())
            }
            ast::Port::InvPort { invoke, name } => {
                let inv = self.get_inv(&invoke)?;
                let owner = InvPort::Inv(inv, dir, name);
                ir::Access::port(self.get_port(&owner)?, self.comp())
            }
            ast::Port::Bundle { name, access } => {
                // NOTE(rachit): The AST does not distinguish between bundles
                // defined by the signature and locally defined bundles so we
                // must search both.
                let owner = InvPort::Sig(dir, name.clone());
                let port = if let Some(p) = self.find_port(&owner) {
                    p
                } else {
                    let owner = InvPort::Local(name);
                    self.get_port(&owner)?
                };
                let (start, end) = self.access(access.take())?;
                ir::Access { port, start, end }
            }
            ast::Port::InvBundle {
                invoke,
                port,
                access,
            } => {
                let inv = self.get_inv(&invoke)?;
                let owner = InvPort::Inv(inv, dir, port);
                let port = self.get_port(&owner)?;
                let (start, end) = self.access(access.take())?;
                ir::Access { port, start, end }
            }
            ast::Port::Constant(_) => todo!("Constant ports"),
        };
        Ok(acc)
    }

    fn sig(&mut self, sig: &ast::Signature) -> BuildRes<Vec<ir::Command>> {
        for param in &sig.params {
            self.param(param.inner(), ir::ParamOwner::Sig);
        }

        // Binding from let-defined parameters in the signature to their values
        for pb in &sig.sig_bindings {
            let ast::ParamBind { default, .. } = pb.inner();
            let e = self.expr(default.as_ref().unwrap().clone())?;
            self.add_let_param(pb.name(), e);
        }

        let mut interface_signals: HashMap<_, _> = sig
            .interface_signals
            .iter()
            .cloned()
            .map(|ast::InterfaceDef { name, event }| (event, name.split()))
            .collect();

        // Declare the events first
        for event in &sig.events {
            // can remove here as each interface signal should only be used once
            let interface = interface_signals.remove(event.event.inner());
            self.declare_event(event.inner(), interface);
        }
        // Then define their delays correctly
        for event in &sig.events {
            let delay = self.timesub(event.inner().delay.inner().clone())?;
            let idx = self.get_event(&event.inner().event)?;
            self.comp().get_mut(idx).delay = delay;
        }
        for port in sig.inputs() {
            // XXX(rachit): Unnecessary clone.
            self.port(port.inner().clone(), ir::PortOwner::sig_out())?;
        }
        for port in sig.outputs() {
            // XXX(rachit): Unnecessary clone.
            self.port(port.inner().clone(), ir::PortOwner::sig_in())?;
        }
        for (name, width) in &sig.unannotated_ports {
            self.comp().unannotated_ports.push((*name, *width));
        }
        // Constraints defined by the signature
        let mut cons = Vec::with_capacity(
            sig.param_constraints.len() + sig.event_constraints.len(),
        );
        for ec in &sig.event_constraints {
            let info = self.comp().add(ir::Info::assert(
                ir::info::Reason::misc("Signature assumption", ec.pos()),
            ));
            let prop = self.event_cons(ec.inner().clone())?;
            cons.extend(self.comp().assume(prop, info));
        }
        for pc in &sig.param_constraints {
            let info = self.comp().add(ir::Info::assert(
                ir::info::Reason::misc("Signature assumption", pc.pos()),
            ));
            let prop = self.expr_cons(pc.inner().clone())?;
            cons.extend(self.comp().assume(prop, info));
        }

        Ok(cons)
    }

    fn instance(&mut self, inst: ast::Instance) -> BuildRes<Vec<ir::Command>> {
        let comp_loc = inst.component.pos();
        // Add the facts defined by the instance as assertions in the
        // component.
        let idx = self.get_inst(&inst.name)?;
        let (binding, component) = self.inst_to_sig.get(idx).clone();
        let facts = self
            .get_sig(&component)?
            .param_cons
            .clone()
            .into_iter()
            .map(|f| {
                let reason = self.comp().add(
                    ir::info::Reason::param_cons(comp_loc, f.pos()).into(),
                );
                let p = f.take().resolve_expr(&binding);
                // This is a checked fact because the calling component needs to
                // honor it.
                self.expr_cons(p).map(|p| self.comp().assert(p, reason))
            })
            .collect::<BuildRes<Vec<_>>>()?
            .into_iter()
            .flatten();

        Ok(iter::once(ir::Command::from(idx))
            .chain(facts)
            .collect_vec())
    }

    /// This function is called during the second pass of the conversion and does the following:
    /// * Defines the input ports of the invocation
    /// * Generate event bindings implied by the invocation
    /// * Generates the connections implied by the arguments to the invoke
    fn invoke(&mut self, inv: ast::Invoke) -> BuildRes<Vec<ir::Command>> {
        let ast::Invoke {
            name,
            abstract_vars,
            ports,
            instance,
        } = inv;
        let Some(ports) = ports else {
            unreachable!("No ports provided for invocation {name}")
        };
        let inv = self.get_inv(&name)?;
        let inst = inv.inst(self.comp());
        let (param_binding, comp) = self.inst_to_sig.get(inst).clone();
        let sig = self.get_sig(&comp)?;
        // foreign component being invoked
        let foreign_comp = inv.comp(self.comp());

        // Event bindings
        let event_binding = sig.event_binding(
            abstract_vars.iter().map(|v| v.inner().clone()),
            &instance,
            self.diag(),
        )?;

        let srcs = ports
            .into_iter()
            .map(|p| p.try_map(|p| self.get_access(p, ir::Direction::Out)))
            .collect::<BuildRes<Vec<_>>>()?;

        assert!(
            sig.inputs.len() == srcs.len(),
            "signature defined {} inputs but provided {} arguments",
            sig.inputs.len(),
            srcs.len()
        );

        // Constraints on the events from the signature
        let cons: Vec<ir::Command> = sig
            .event_cons
            .clone()
            .into_iter()
            .map(|ec| {
                let reason = self.comp().add(
                    ir::info::Reason::event_cons(instance.pos(), ec.pos())
                        .into(),
                );
                let ec = ec.take().resolve_event(&event_binding);
                self.event_cons(ec)
                    .map(|prop| self.comp().assert(prop, reason))
            })
            .collect::<BuildRes<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();

        let mut connects = Vec::with_capacity(sig.inputs.len());

        for (idx, (p, src)) in
            sig.raw_inputs.clone().into_iter().zip(srcs).enumerate()
        {
            let info = self
                .comp()
                .add(ir::Info::connect(p.inner().name().pos(), src.pos()));
            let resolved = p.map(|p| {
                p.resolve_exprs(&param_binding)
                    .resolve_event(&event_binding)
            });

            let base = ir::Foreign::new(
                self.sig_from_idx(foreign_comp).inputs[idx],
                foreign_comp,
            );

            let owner = ir::PortOwner::Inv {
                inv,
                dir: ir::Direction::In,
                base,
            };

            // Define port and add it to the invocation
            let pidx = self.port(resolved.take(), owner)?;
            self.comp().get_mut(inv).ports.push(pidx);

            let end = self.comp()[pidx].live.len;
            let dst = ir::Access {
                port: pidx,
                start: self.comp().num(0),
                end,
            };

            connects.push(
                ir::Connect {
                    src: src.take(),
                    dst,
                    info,
                }
                .into(),
            )
        }

        // Events defined by the invoke
        sig.raw_events
            .iter()
            .zip_longest(abstract_vars.iter())
            .map(|pair| match pair {
                itertools::EitherOrBoth::Both(evt, t) => {
                    (evt, t.inner(), t.pos())
                }
                itertools::EitherOrBoth::Left(evt) => (
                    evt,
                    event_binding.get(evt.event.inner()),
                    GPosIdx::UNKNOWN,
                ),
                itertools::EitherOrBoth::Right(_) => {
                    unreachable!("More arguments than events.")
                }
            })
            .enumerate()
            .try_for_each(|(idx, (event, time, pos))| -> BuildRes<()> {
                let ev_delay_loc = event.delay.pos();
                let resolved = event
                    .clone()
                    .resolve_exprs(&param_binding)
                    .resolve_event(&event_binding);

                let info =
                    self.comp().add(ir::Info::event_bind(ev_delay_loc, pos));
                let arg = self.time(time.clone())?;
                let event = self.timesub(resolved.delay.take())?;
                let base = ir::Foreign::new(EventIdx::new(idx), foreign_comp);
                let eb = ir::EventBind::new(event, arg, info, base);
                let invoke = self.comp().get_mut(inv);
                invoke.events.push(eb);
                Ok(())
            })?;

        Ok(std::iter::once(ir::Command::from(inv))
            .chain(connects)
            .chain(cons)
            .collect_vec())
    }

    fn commands(
        &mut self,
        cmds: Vec<ast::Command>,
    ) -> BuildRes<Vec<ir::Command>> {
        self.declare_cmds(&cmds)?;
        let mut new_cmds: Vec<ir::Command> = Vec::with_capacity(cmds.len());
        for cmd in cmds {
            new_cmds.extend(self.command(cmd)?);
        }
        Ok(new_cmds)
    }

    fn command(&mut self, cmd: ast::Command) -> BuildRes<Vec<ir::Command>> {
        let cmds = match cmd {
            ast::Command::Invoke(inv) => self.invoke(inv)?,
            ast::Command::Instance(inst) => self.instance(inst)?,
            ast::Command::Fact(ast::Fact { cons, checked }) => {
                let reason = self.comp().add(
                    ir::info::Reason::misc("source-level fact", cons.pos())
                        .into(),
                );
                let prop = self.implication(cons.take())?;
                let fact = if checked {
                    self.comp().assert(prop, reason)
                } else {
                    self.comp().assume(prop, reason)
                };
                fact.into_iter().collect()
            }
            ast::Command::Connect(ast::Connect { src, dst }) => {
                let info =
                    self.comp().add(ir::Info::connect(dst.pos(), src.pos()));
                let src = self.get_access(src.take(), ir::Direction::Out)?;
                let dst = self.get_access(dst.take(), ir::Direction::In)?;
                vec![ir::Connect { src, dst, info }.into()]
            }
            ast::Command::ParamLet(_) => {
                // The declare phase already added the rewrite for this binding
                vec![]
            }
            ast::Command::ForLoop(ast::ForLoop {
                idx,
                start,
                end,
                body,
            }) => {
                let start = self.expr(start)?;
                let end = self.expr(end)?;
                // Assumption that the index is within range
                let reason = self.comp().add(
                    ir::info::Reason::misc(
                        "loop index is within range",
                        idx.pos(),
                    )
                    .into(),
                );

                // Compile the body in a new scope
                let (index, body) = self.try_with_scope(|this| {
                    let idx = this.param(
                        &ast::ParamBind::from(idx),
                        ir::ParamOwner::Loop,
                    );
                    Ok((idx, this.commands(body)?))
                })?;
                let l = ir::Loop {
                    index,
                    start,
                    end,
                    body,
                }
                .into();
                let index = index.expr(self.comp());
                let idx_start = index.gte(start, self.comp());
                let idx_end = index.lt(end, self.comp());
                let in_range = idx_start.and(idx_end, self.comp());
                iter::once(l)
                    .chain(self.comp().assume(in_range, reason))
                    .collect()
            }
            ast::Command::If(ast::If { cond, then, alt }) => {
                let cond = self.expr_cons(cond)?;
                let then = self.commands(then)?;
                let alt = self.commands(alt)?;
                vec![ir::If { cond, then, alt }.into()]
            }
            ast::Command::Bundle(bun) => {
                // Add the bundle to the current scope
                let idx =
                    self.port(ast::PortDef::Bundle(bun), ir::PortOwner::Local)?;
                vec![ir::Command::from(idx)]
            }
        };
        Ok(cmds)
    }

    /// Adds assumptions about the ports in the component
    fn port_assumptions(&mut self) -> Vec<ir::Command> {
        let mut cmds = Vec::with_capacity(self.comp().ports().len() * 2);
        let ports = self
            .comp()
            .ports()
            .iter()
            .map(|(_, p)| (p.live.idx, p.live.len))
            .collect_vec();
        // Add assumptions for range of bundle-bound indices
        let reason = self.comp().add(
            ir::info::Reason::misc(
                "bundle index is within range",
                GPosIdx::UNKNOWN,
            )
            .into(),
        );

        for (idx, len) in ports {
            let idx = idx.expr(self.comp());
            let start = idx.gte(self.comp().num(0), self.comp());
            let end = idx.lt(len, self.comp());
            let in_range = start.and(end, self.comp());
            cmds.extend(self.comp().assume(in_range, reason))
        }
        cmds
    }
}

fn try_transform(ns: ast::Namespace) -> BuildRes<ir::Context> {
    // creates an empty context with the main index.
    let mut ctx = ir::Context {
        entrypoint: ns
            .main_idx()
            // index main component after all externals
            .map(|idx| Idx::new(ns.externals().count() + idx)),
        ..Default::default()
    };

    // Walk over signatures and compile signatures to build a SigMap
    // Contains a tuple containing three necessary bits of information:
    // 1. The (optional) name of the component (if it is an external)
    // 2. The signature of the component
    // 3. The (optional) body of the component (if it is not an external)
    let comps = ns
        // pull signatures out of externals
        .externs
        .into_iter()
        .flat_map(|(name, comps)| {
            comps.into_iter().map(move |comp| (name.clone(), comp))
        })
        .map(|(name, sig)| (Some(name), sig, None))
        // add signatures of components as well as their command bodies
        .chain(
            ns.components
                .into_iter()
                .map(|comp| (None, comp.sig, Some(comp.body))),
        )
        .enumerate();

    // used in the beginning so signatures of components can be built without any information
    let sig_map = SigMap::default();

    /// Container to store build information after compiling a signature.
    struct Builder<'a> {
        idx: ir::CompIdx,
        builder: BuildCtx<'a>,
        body: Option<Vec<ast::Command>>,
    }

    // uses the information above to compile the signatures of components and create their builders.
    let (mut builders, sig_map): (Vec<_>, SigMap) = comps
        .map(|(idx, (file, sig, body))| {
            let idx = ir::CompIdx::new(idx);
            let mut builder =
                BuildCtx::new(ir::Component::new(body.is_none()), &sig_map);

            // enable source information saving if this is main or an external.
            if body.is_none() || Some(idx) == ctx.entrypoint {
                builder.comp().src_info =
                    Some(InterfaceSrc::new(sig.name.copy()))
            }
            // add the file to the externals map if it exists
            if let Some(file) = file {
                ctx.externals.entry(file).or_default().push(idx);
            }

            // compile the signature
            builder.comp().cmds = builder.sig(&sig)?;

            let irsig = Sig::new(idx, builder.comp(), &sig);
            Ok((Builder { idx, builder, body }, (sig.name.take(), irsig)))
        })
        .collect::<BuildRes<Vec<_>>>()?
        .into_iter()
        .unzip();

    // Add the signature map to each builder
    builders.iter_mut().for_each(|Builder { builder, .. }| {
        builder.set_sig_map(&sig_map);
    });

    // Compiles and adds all the commands here
    // Need to do this before adding all the components because each builder borrows the context immutably
    for Builder {
        idx,
        mut builder,
        body,
    } in builders
    {
        let body_cmds = match body {
            Some(cmds) => builder.commands(cmds)?,
            None => vec![],
        };
        let mut cmds = builder.port_assumptions();
        cmds.extend(body_cmds);
        builder.comp().cmds.extend(cmds);
        log::debug!("Adding component: {}", idx);
        ctx.comps.checked_add(idx, builder.take())
    }

    Ok(ctx)
}

pub fn transform(ns: ast::Namespace) -> Result<ir::Context, u64> {
    match try_transform(ns) {
        Ok(ctx) => Ok(ctx),
        Err(mut e) => Err(e.report_all().unwrap()),
    }
}
