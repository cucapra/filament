//! Convert the frontend AST to the IR.
use super::build_ctx::{OwnedParam, OwnedPort};
use super::{BuildCtx, Sig, SigMap};
use crate::utils::Idx;
use crate::{self as ir, EntryPoint};
use crate::{
    AddCtx, Cmp, Ctx, DisplayCtx, EventIdx, ExprIdx, InterfaceSrc, MutCtx,
    ParamIdx, PortIdx, PropIdx, TimeIdx,
};
use fil_ast::{self as ast};
use fil_utils::{Diagnostics, Error, GPosIdx};
use itertools::Itertools;
use std::collections::HashMap;
use std::rc::Rc;

pub type BuildRes<T> = Result<T, Diagnostics>;

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
impl BuildCtx<'_> {
    fn declare_inst(&mut self, inst: &ast::Instance) -> BuildRes<()> {
        let ast::Instance {
            name,
            component,
            params: bindings,
            lives,
        } = inst;

        let comp = self.get_sig(component)?;
        let mut binding = comp.param_binding(
            bindings.iter().map(|e| e.inner()).cloned().collect_vec(),
            component.clone(),
            self.diag(),
        )?;
        let mut live_locs = Vec::with_capacity(lives.len());
        lives.iter().for_each(|l| {
            let (_, pos) = l.clone().split();
            live_locs.push(pos);
        });
        let inst = ir::Instance {
            comp: comp.idx,
            args: binding
                .iter()
                .map(|(_, b)| self.expr(b.clone()))
                .collect::<BuildRes<Vec<_>>>()?
                .into_boxed_slice(),
            params: Vec::default(), // Filled in when we iterate over sig_bindings
            info: self.comp().add(ir::Info::instance(
                name.copy(),
                component.pos(),
                name.pos(),
                live_locs,
            )),
            lives: Vec::default(), // fill this in later so we can reference the instance
        };

        let idx = self.comp().add(inst);

        // Extend the binding with the let-bound parameters in the signature
        // Bindings can mention previous bindings so we add bindings as we
        // go along.
        let params = comp
            .sig_binding
            .iter()
            .flat_map(|(sb, mb_param)| match sb {
                ast::SigBind::Let { param, bind } => {
                    let e = bind.clone().resolve(&binding);
                    binding.extend([(param.copy(), e)]);
                    None
                }
                ast::SigBind::Exists { param, .. } => {
                    // Define the parameter in the component
                    let p_idx = mb_param.unwrap_or_else(|| {
                        unreachable!(
                            "existential binding must define a parameter"
                        )
                    });
                    let base = ir::Foreign::new(p_idx, comp.idx);
                    let p_idx = self.param(
                        param.clone(),
                        ir::ParamOwner::Instance { inst: idx, base },
                    );
                    // Map this parameter to the instance's version of the parameter.
                    // This is because we want it to resolve the parameter to the one defined in this component.
                    let e = ast::Expr::ParamAccess {
                        inst: name.clone(),
                        param: param.clone(),
                    };
                    binding.extend([(param.copy(), e)]);
                    Some(p_idx)
                }
            })
            .collect_vec();

        let inst = self.comp().get_mut(idx);
        inst.params.extend(params);

        self.add_inst(name.copy(), idx);
        // Track the component binding for this instance

        // fill in lives here after we've added the instance name
        let lives = lives
            .iter()
            .map(|l| {
                let (l, _) = l.clone().split();
                self.range(l)
            })
            .collect::<BuildRes<Vec<_>>>()?;

        let inst = self.comp().get_mut(idx);
        inst.lives.extend(lives);

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
        let empty = self.comp().add(ir::Info::empty());
        let inv = self.comp().add(ir::Invoke {
            inst,
            ports: vec![],  // Filled in later
            events: vec![], // Filled in later
            info: empty,    // Filled in later
        });
        // foreign component being invoked
        let foreign_comp = inv.comp(self.comp());
        self.add_inv(name.copy(), inv);

        let mut def_ports = vec![];

        // The inputs
        let (param_binding, comp) = self.inst_to_sig.get(inst).clone();
        let sig = self.get_sig(&comp)?;

        let mut event_bind_locs = Vec::with_capacity(abstract_vars.len());
        // Event bindings
        let event_binding = sig.event_binding(
            abstract_vars
                .iter()
                .map(|v| {
                    let (v, pos) = v.clone().split();
                    event_bind_locs.push(pos);
                    v
                })
                .collect_vec(),
            instance,
            self.diag(),
        )?;

        // Define the output port from the invoke
        for (p, idx) in sig.outputs.clone().into_iter() {
            let resolved = p
                .take()
                .resolve_exprs(&param_binding)
                .resolve_event(&event_binding);

            let base = ir::Foreign::new(idx, foreign_comp);

            let owner = ir::PortOwner::Inv {
                inv,
                dir: ir::Direction::Out,
                base,
            };
            def_ports.push(self.port(resolved, owner)?);
        }

        let info = self.comp().add(ir::Info::invoke(
            name.copy(),
            instance.pos(),
            name.pos(),
            event_bind_locs,
        ));
        let inv = self.comp().get_mut(inv);
        // Update the information
        inv.info = info;
        // Add the inputs from the invoke. The outputs are added in the second
        // pass over the AST.
        inv.ports.extend(def_ports);

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
                let owner = ir::ParamOwner::Let {
                    bind: match expr {
                        Some(expr) => Some(self.expr(expr.clone())?),
                        None => None,
                    },
                };
                self.param(name.clone(), owner);
                Ok(())
            }
            ast::Command::Exists(_) => {
                /* The existential parameter is already defined so we can resolve
                 * this in the second pass */
                Ok(())
            }
            ast::Command::ForLoop(_) => {
                /* The index parameter is bound when we enter an for loop so we
                 * don't have to do it here. */
                Ok(())
            }
            ast::Command::If(_)
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

impl BuildCtx<'_> {
    fn expr(&mut self, expr: ast::Expr) -> BuildRes<ExprIdx> {
        let expr = match expr {
            ast::Expr::Abstract(p) => {
                self.get_param(&OwnedParam::local(p.copy()), p.pos())?
            }
            ast::Expr::ParamAccess { inst, param } => {
                let inst_idx = self.get_inst(&inst)?;
                self.get_param(
                    &OwnedParam::inst(inst_idx, param.copy()),
                    param.pos(),
                )?
            }
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
            ast::Expr::App { func, args } => {
                let args = args
                    .into_iter()
                    .map(|arg| self.expr(arg))
                    .collect::<Result<Vec<_>, _>>()?;
                // The .add call simplifies the expression if possible
                self.comp().add(ir::Expr::Fn { op: func, args })
            }
            ast::Expr::If { cond, then, alt } => {
                let ast::OrderConstraint { left, right, op } = cond;
                let cond = self.expr_cons(ast::OrderConstraint {
                    left: *left,
                    right: *right,
                    op,
                })?;
                let then = self.expr(*then)?;
                let alt = self.expr(*alt)?;
                self.comp().add(ir::Expr::If { cond, then, alt })
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
        param: ast::Loc<ast::Id>,
        owner: ir::ParamOwner,
    ) -> ParamIdx {
        let (name, pos) = param.split();
        let info = self.comp().add(ir::Info::param(name, pos));
        let owned = OwnedParam::param_owner(name, &owner);
        let is_sig_owned = matches!(
            owner,
            ir::ParamOwner::Sig | ir::ParamOwner::Exists { .. }
        );
        let ir_param = ir::Param::new(owner, info);
        let idx = self.comp().add(ir_param);
        let e_idx = idx.expr(self.comp());
        self.add_param_map(owned, e_idx);

        // only add information if this is a signature defined parameter
        if is_sig_owned {
            // If the component is expecting interface information, add it.
            if let Some(src) = &mut self.comp().src_info {
                src.params.push(idx, name);
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
                src.interface_ports.push(idx, name);
            }
            // add event name (used by dump_interface)
            src.events.push(idx, *eb.event);
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
        let ast::Bundle {
            name,
            typ:
                ast::BundleType {
                    idx,
                    len,
                    liveness,
                    bitwidth,
                },
            attrs,
        } = pd;

        let info = self.comp().add(ir::Info::port(
            name.copy(),
            name.pos(),
            bitwidth.pos(),
            liveness.pos(),
        ));
        // Construct the bundle type in a new scope.
        let live = self.try_with_scope(|ctx| {
            Ok(ir::Liveness {
                idxs: idx
                    .into_iter()
                    .map(|idx| {
                        ctx.param(
                            // Updated after the port is constructed
                            idx,
                            ir::ParamOwner::bundle(PortIdx::UNKNOWN),
                        )
                    })
                    .collect_vec(),
                lens: len
                    .into_iter()
                    .map(|len| ctx.expr(len.take()))
                    .collect::<BuildRes<Vec<_>>>()?,
                range: ctx.range(liveness.take())?,
            })
        })?;
        let p = ir::Port {
            width: self.expr(bitwidth.take())?,
            owner,
            live,
            info,
        };

        // Defines helper variable here due to lifetime issues
        let is_sig_port = p.is_sig();
        let idx = self.comp().add(p);
        // Add the attributes to port attributes
        self.comp().port_attrs.push(idx, attrs);
        // Fixup the liveness index parameter's owner
        let idxs = self.comp().get(idx).live.idxs.clone();
        for p in idxs {
            let param = self.comp().get_mut(p);
            param.owner = ir::ParamOwner::bundle(idx);
        }

        // If this is a signature port, try adding it to the component's external interface
        if is_sig_port {
            // If the component is expecting interface information, add it.
            if let Some(src) = &mut self.comp().src_info {
                src.ports.push(idx, name.copy());
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
        let acc = match (&port.base, &port.access[..]) {
            (ast::PortRef::This { port: name }, []) => {
                // NOTE: The AST does not distinguish between ports
                // defined by the signature and locally defined ports so we
                // must search both.
                let owner = OwnedPort::Sig(dir, name.clone());
                let port = if let Some(port) = self.find_port(&owner) {
                    port
                } else {
                    let owner = OwnedPort::Local(name.clone());
                    self.get_port(&owner)?
                };

                ir::Access::port(port, self.comp())
            }
            (
                ast::PortRef::Instance {
                    instance: invoke,
                    port: name,
                },
                [],
            ) => {
                let inv = self.get_inv(invoke)?;
                let owner = OwnedPort::Inv(inv, dir, name.clone());
                ir::Access::port(self.get_port(&owner)?, self.comp())
            }
            (ast::PortRef::This { port: name }, access) => {
                // NOTE(rachit): The AST does not distinguish between bundles
                // defined by the signature and locally defined bundles so we
                // must search both.
                let owner = OwnedPort::Sig(dir, name.clone());
                let port = if let Some(p) = self.find_port(&owner) {
                    p
                } else {
                    let owner = OwnedPort::Local(name.clone());
                    self.get_port(&owner)?
                };
                let ranges = access
                    .iter()
                    .map(|a| self.access(a.clone().take()))
                    .collect::<BuildRes<Vec<_>>>()?;
                ir::Access { port, ranges }
            }
            (
                ast::PortRef::Instance {
                    instance: invoke,
                    port,
                },
                access,
            ) => {
                let inv = self.get_inv(invoke)?;
                let owner = OwnedPort::Inv(inv, dir, port.clone());
                let port = self.get_port(&owner)?;
                let ranges = access
                    .iter()
                    .map(|a| self.access(a.clone().take()))
                    .collect::<BuildRes<Vec<_>>>()?;
                ir::Access { port, ranges }
            }
        };
        Ok(acc)
    }

    fn sig(&mut self, idx: ir::CompIdx, sig: &ast::Signature) -> BuildRes<Sig> {
        let mut conv_sig = Sig::new(idx, sig);

        // Add parameters to the component
        self.comp().param_args = sig
            .params
            .iter()
            .map(|pb| self.param(pb.param.clone(), ir::ParamOwner::Sig))
            .collect_vec()
            .into_boxed_slice();

        // Binding from let-defined parameters in the signature to their values
        conv_sig.sig_binding = sig
            .sig_bindings
            .iter()
            .map(|sb| {
                match &sb.inner() {
                    ast::SigBind::Let { param, bind } => {
                        let e = self.expr(bind.clone())?;
                        self.add_param_rewrite(param.copy(), e);
                        Ok((sb.inner().clone(), None))
                    }
                    ast::SigBind::Exists {
                        param,
                        opaque,
                        cons,
                    } => {
                        let p_idx = self.param(
                            param.clone(),
                            ir::ParamOwner::Exists { opaque: *opaque },
                        );
                        // Constraints on existentially quantified parameters
                        let assumes = cons
                            .iter()
                            .map(|pc| {
                                self.expr_cons(pc.inner().clone())
                                    .map(|p| (p, pc.pos()))
                            })
                            .collect::<BuildRes<Vec<_>>>()?;
                        self.comp().add_exist_assumes(p_idx, assumes);
                        Ok((sb.inner().clone(), Some(p_idx)))
                    }
                }
            })
            .collect::<BuildRes<Vec<_>>>()?;

        let mut interface_signals: HashMap<_, _> = sig
            .interface_signals
            .iter()
            .cloned()
            .map(|ast::InterfaceDef { name, event }| (event, name.split()))
            .collect();

        // Declare the events first
        self.comp().event_args = sig
            .events
            .iter()
            .map(|event| {
                // can remove here as each interface signal should only be used once
                let interface = interface_signals.remove(event.event.inner());
                self.declare_event(event.inner(), interface)
            })
            .collect_vec()
            .into_boxed_slice();

        // Then define their delays correctly
        for event in &sig.events {
            let delay = self.timesub(event.inner().delay.inner().clone())?;
            let idx = self.get_event(&event.inner().event)?;
            self.comp().get_mut(idx).delay = delay;
        }
        for port in sig.inputs() {
            // XXX(rachit): Unnecessary clone.
            let idx =
                self.port(port.inner().clone(), ir::PortOwner::sig_out())?;
            conv_sig.inputs.push((port.clone(), idx));
        }
        for port in sig.outputs() {
            // XXX(rachit): Unnecessary clone.
            let idx =
                self.port(port.inner().clone(), ir::PortOwner::sig_in())?;
            conv_sig.outputs.push((port.clone(), idx));
        }
        for (name, width) in &sig.unannotated_ports {
            self.comp().unannotated_ports.push((*name, *width));
        }
        // Constraints defined by the signature
        for ec in &sig.event_constraints {
            let prop = self.event_cons(ec.inner().clone())?;
            self.comp().add_event_assert([(prop, ec.pos())]);
        }
        for pc in &sig.param_constraints {
            let prop = self.expr_cons(pc.inner().clone())?;
            self.comp().add_param_assert([(prop, pc.pos())]);
        }

        Ok(conv_sig)
    }

    fn instance(&mut self, inst: ast::Instance) -> BuildRes<Vec<ir::Command>> {
        let idx = self.get_inst(&inst.name)?;
        Ok(vec![ir::Command::from(idx)])
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

        if sig.inputs.len() != ports.len() {
            let msg = format!(
                "instance `{}' requires {} inputs but provided {} arguments",
                instance.copy(),
                sig.inputs.len(),
                ports.len()
            );
            let info = self.diag().add_info(msg.clone(), instance.pos());
            let err = Error::malformed(msg);

            return self.fail(err, [info]);
        }

        let srcs = ports
            .into_iter()
            .map(|p| p.try_map(|p| self.get_access(p, ir::Direction::Out)))
            .collect::<BuildRes<Vec<_>>>()?;

        let mut connects = Vec::with_capacity(sig.inputs.len());

        for ((p, idx), src) in sig.inputs.clone().into_iter().zip(srcs) {
            let info = self
                .comp()
                .add(ir::Info::connect(p.inner().name.pos(), src.pos()));
            let resolved = p.map(|p| {
                p.resolve_exprs(&param_binding)
                    .resolve_event(&event_binding)
            });

            let base = ir::Foreign::new(idx, foreign_comp);

            let owner = ir::PortOwner::Inv {
                inv,
                dir: ir::Direction::In,
                base,
            };

            // Define port and add it to the invocation
            let pidx = self.port(resolved.take(), owner)?;
            self.comp().get_mut(inv).ports.push(pidx);

            let zero = self.comp().num(0);
            let ranges = self.comp()[pidx]
                .live
                .lens
                .iter()
                .map(|end| (zero, *end))
                .collect_vec();
            let dst = ir::Access { port: pidx, ranges };

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
            ast::Command::Exists(ast::Exists { param, bind }) => {
                let expr = self.expr(bind.inner().clone())?;
                let owner = OwnedParam::Local(param.copy());
                let param_expr = self.get_param(&owner, param.pos())?;
                let Some(p_idx) = param_expr.as_param(self.comp()) else {
                    unreachable!(
                        "Existing LHS is an expression: {}",
                        self.comp().display(param_expr)
                    )
                };
                // Ensure that the parameter is an existential parameter
                if !matches!(
                    self.comp().get(p_idx).owner,
                    ir::ParamOwner::Exists { .. }
                ) {
                    let diag = self.diag();
                    let param_typ = Error::malformed("parameter in exists binding is not existentially quantified").add_note(
                        diag.add_info("parameter is not existentially quantified", param.pos()),
                    );
                    diag.add_error(param_typ);
                    return Err(std::mem::take(diag));
                }

                vec![ir::Exists { param: p_idx, expr }.into()]
            }
            ast::Command::Fact(ast::Fact { cons, checked }) => {
                let reason = self.comp().add(
                    ir::info::Reason::misc(
                        "cannot prove source-level fact",
                        cons.pos(),
                    )
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
            ast::Command::ParamLet(ast::ParamLet { name, expr }) => {
                let param = self
                    .get_param(&OwnedParam::Local(name.copy()), name.pos())?;
                let ir::Expr::Param(param) = *self.comp().get(param) else {
                    unreachable!(
                        "let-bound parameter was rewritten to expression"
                    )
                };
                let expr = match expr {
                    Some(e) => Some(self.expr(e)?),
                    None => None,
                };
                // The declare phase already added the rewrite for this binding
                vec![ir::Let { param, expr }.into()]
            }
            ast::Command::ForLoop(ast::ForLoop {
                idx,
                start,
                end,
                body,
            }) => {
                let start = self.expr(start)?;
                let end = self.expr(end)?;

                // Compile the body in a new scope
                let (index, body) = self.try_with_scope(|this| {
                    let idx = this.param(idx, ir::ParamOwner::Loop);
                    Ok((idx, this.commands(body)?))
                })?;

                vec![
                    ir::Loop {
                        index,
                        start,
                        end,
                        body,
                    }
                    .into(),
                ]
            }
            ast::Command::If(ast::If { cond, then, alt }) => {
                let cond = self.expr_cons(cond)?;
                let then = self.try_with_scope(|this| this.commands(then))?;
                let alt = self.try_with_scope(|this| this.commands(alt))?;
                vec![ir::If { cond, then, alt }.into()]
            }
            ast::Command::Bundle(bun) => {
                // Add the bundle to the current scope
                let idx = self.port(bun, ir::PortOwner::Local)?;
                vec![ir::Command::from(idx)]
            }
        };
        Ok(cmds)
    }
}

/// Information about generated components
enum TypeInfo {
    /// Contains source information
    Source(Vec<ast::Command>),
    /// Contains extern verilog module name
    External(String),
    /// Contains extern tool name
    Generated(String),
}

/// Temporary state used to store information from the component
/// before the IR version is created.
struct ComponentTransform {
    /// Type of the component
    typ: TypeInfo,
    /// Signature of the component
    sig: ast::Signature,
}

/// Convert an [ast::Component] into a source [ComponentTransform]
impl From<ast::Component> for ComponentTransform {
    fn from(comp: ast::Component) -> Self {
        Self {
            typ: TypeInfo::Source(comp.body),
            sig: comp.sig,
        }
    }
}

/// Convert an [ast::Extern] into an external/generated [ComponentTransform]
impl From<ast::Extern> for ComponentTransform {
    fn from(ext: ast::Extern) -> Self {
        let typ = if ext.gen_tool.is_none() {
            TypeInfo::External(ext.path)
        } else {
            TypeInfo::Generated(ext.gen_tool.unwrap())
        };
        Self {
            typ,
            sig: ast::Signature::default(),
        }
    }
}

fn try_transform(ns: ast::Namespace) -> BuildRes<ir::Context> {
    // creates an empty context with the main index.
    let mut ctx = ir::Context {
        entrypoint: ns
            .main_idx()
            // index main component after all externals
            .map(|idx| Idx::new(ns.externals().count() + idx))
            .map(EntryPoint::new),
        ..Default::default()
    };

    // Loc<Id> of the toplevel component
    let toplevel_id =
        ns.main_idx().map(|pos| ns.components[pos].sig.name.clone());

    // Walk over signatures and compile signatures to build a SigMap
    // Contains a tuple containing three necessary bits of information:
    // 1. Component type (external, gen, or source)
    // 2. The (optional) name of the component (if it is an external)
    // 3. Attributes of the component
    // 4. The signature of the component
    // 5. The (optional) body of the component (if it is not an external)
    let comps = ns
        // pull signatures out of externals
        .externs
        .into_iter()
        // track (extern location / gen tool name, signature, body)
        .flat_map(
            |ast::Extern {
                 comps,
                 gen_tool,
                 path,
             }| {
                comps.into_iter().map(move |comp| {
                    let typ = if let Some(name) = &gen_tool {
                        TypeInfo::Generated(name.clone())
                    } else {
                        TypeInfo::External(path.clone())
                    };
                    ComponentTransform { typ, sig: comp }
                })
            },
        )
        // add signatures of components as well as their command bodies
        .chain(ns.components.into_iter().map(ComponentTransform::from))
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
        .map(|(idx, comp_ctx)| {
            let idx = ir::CompIdx::new(idx);
            let mut builder = BuildCtx::new(ir::Component::new(
                match comp_ctx.typ {
                    TypeInfo::Source(_) => ir::CompType::Source,
                    TypeInfo::External(_) => ir::CompType::External,
                    TypeInfo::Generated(_) => ir::CompType::Generated,
                }, comp_ctx.sig.attributes.clone()), &sig_map);

            // enable source information saving if this is main
            if ctx.is_main(idx) {
                builder.comp().src_info =
                    Some(InterfaceSrc::new(comp_ctx.sig.name.copy(), None))
            }
            // add the file to the externals map if it exists
            match &comp_ctx.typ {
                TypeInfo::External(path) => {
                    ctx.externals.entry(path.clone()).or_default().push(idx);

                    builder.comp().src_info = Some(InterfaceSrc::new(comp_ctx.sig.name.copy(), None));
                }
                TypeInfo::Generated(name) => {
                    builder.comp().src_info = Some(InterfaceSrc::new(comp_ctx.sig.name.copy(), Some(name.clone())));
                }
                _ => {}

            };

            // compile the signature
            let irsig = builder.sig(idx, &comp_ctx.sig)?;

            // Now that all the signature has been compiled,
            // if this component is the main component, add the provided
            // bindings to the entrypoint
            // this needs to be done here because we need to provide
            // possible default values for the parameters
            if ctx.is_main(idx) {
                let Some(ep) = &mut ctx.entrypoint else {
                    unreachable!(
                        "Entrypoint not set despite main component existing"
                    );
                };

                let Some(toplevel_id) = &toplevel_id else {
                    unreachable!("Main component had no name")
                };

                ep.bindings = irsig
                    .param_binding(
                        ns.bindings.iter().copied().map(ast::Expr::Concrete),
                        toplevel_id.clone(),
                        builder.diag(),
                    ).map_err(
                        |mut e| {
                            let err = Error::misc(format!("Incorrect parameter bindings provided to top-level component {}", comp_ctx.sig.name)).add_note(e.add_message("Parameter bindings should be provided via the `--bindings` flag in a `.toml` file."));
                            e.add_error(err);
                            e
                        }
                    )?
                    .into_iter()
                    .enumerate()
                    .map(|(i, (_, e))| {
                        if let ast::Expr::Concrete(e) = e {
                            Ok(e)
                        } else {
                            let diag = builder.diag();
                            let err = Error::malformed(
                                "Default values for parameters in the main component must be concrete",
                            ).add_note(
                                diag.add_info(
                                    "Parameter was not given a concrete value",
                                    irsig.raw_params[i].pos()
                                )
                            );
                            diag.add_error(err);
                            Err(std::mem::take(diag))
                        }
                    })
                    .collect::<BuildRes<Vec<_>>>()?;
            }

            Ok((Builder {
                idx,
                builder,
                body: match comp_ctx.typ {
                TypeInfo::Source(body) => Some(body),
                _ => None,
            }}, (comp_ctx.sig.name.take(), irsig)))
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
        builder.comp().cmds.extend(body_cmds);
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
