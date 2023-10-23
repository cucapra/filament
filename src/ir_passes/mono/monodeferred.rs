use super::{
    Base, CompKey, IntoUdl, MonoSig, Monomorphize, Underlying, UnderlyingComp,
};
use fil_ir::{self as ir, AddCtx, Ctx};
use ir::DisplayCtx;
use itertools::Itertools;

/// Defines methods required to monomorphize a component. Most of the updates
/// happen to the stored [MonoSig] object which keeps all the maps needed while
/// monomorphizing the component.
pub struct MonoDeferred<'a, 'pass: 'a> {
    /// The underlying component to be monomorphized
    pub underlying: UnderlyingComp<'a>,
    /// Underlying pointer
    pub pass: &'a mut Monomorphize<'pass>,
    /// Struct to keep track of all the mapping information from things owned by
    /// `underlying` to things owned by `base`
    pub monosig: MonoSig,
}

impl MonoDeferred<'_, '_> {
    /// The [CompKey] associated with the underlying component being monomorphized.
    fn comp_key(&self) -> CompKey {
        let binding = self.monosig.binding.inner();
        let conc_params = if self.underlying.is_ext() {
            vec![]
        } else {
            binding
                .iter()
                .filter(|(p, _)| self.underlying.get(*p).is_sig_owned())
                .map(|(_, n)| *n)
                .collect_vec()
        };

        (self.monosig.underlying_idx, conc_params).into()
    }

    // XXX(rachit): Why does this function need to do anything to the signature
    // of external components instead of just wholesale copying them?
    pub fn comp(mut self) -> ir::Component {
        // Events can be recursive, so do a pass over them to generate the new idxs now
        // and then fill them in later
        let comp_k: CompKey = self.comp_key();

        let monosig = &mut self.monosig;
        let ul = &self.underlying;
        let is_ext = ul.is_ext();
        let pass = &mut self.pass;

        for (idx, event) in ul.events().iter() {
            let new_idx = monosig.base.add(event.clone());
            monosig.event_map.insert(idx.ul(), new_idx);
            pass.inst_info_mut(comp_k.clone())
                .add_event(idx.ul(), new_idx);
        }

        // Directly add expressions and parameters for external components. For
        // non-external components, the only parameters that will remain in the
        // component are the bundle parameters.
        if is_ext {
            // We can copy over the underlying expressions because we're not
            // going to substitute anything.
            for (_, expr) in ul.exprs().iter() {
                monosig.base.add(expr.clone());
            }

            // Add all parameters because we're not going to substitute them
            for (idx, param) in ul.params().iter() {
                let ir::Param { owner, info } = param;
                let info = info.ul();
                let param = ir::Param {
                    owner: owner.clone(),
                    info: monosig.info(ul, pass, info).get(),
                };
                let new_idx = monosig.base.add(param);
                monosig.param_map.push(idx.ul(), new_idx);
            }
        }

        // Monomorphize port in the signature. Other ports are monomorphized
        // while traversing commands.
        if is_ext {
            for (idx, port) in ul.ports().iter() {
                if port.is_sig() {
                    monosig.ext_port(ul, pass, idx.ul());
                }
            }
        } else {
            for (idx, port) in ul.ports().iter() {
                if port.is_sig() {
                    // Only monomorphize the definition without the data.
                    let port = monosig.port_def_partial(ul, pass, idx.ul());
                    // Add these ports to the global port information for this
                    // instance because other components need this information
                    // to resolve foreigns.
                    pass.inst_info_mut(comp_k.clone()).add_port(idx.ul(), port)
                }
            }
        }

        let src_info = ul.src_info();
        monosig.interface(ul, src_info);

        let unannotated_ports = ul.unannotated_ports().clone();
        monosig.base.set_unannotated_ports(unannotated_ports);

        // Monomorphize the component's body
        for cmd in self.underlying.cmds().clone() {
            let cmd = self.command(&cmd);
            self.monosig.base.extend_cmds(cmd);
        }

        if !is_ext {
            // Extend the binding with existential parameters and monomorphize signature port data
            let info = self.pass.inst_info(&comp_k);
            for param in self.underlying.exist_params() {
                let param = param.ul();
                let Some(v) = info.get_exist_val(param) else {
                    unreachable!(
                        "No binding for existential parameter `{}'. Is the body missing an `exist` assignment?",
                        self.underlying.display(param)
                    )
                };
                self.monosig.binding.push(param, v);
            }

            for (idx, _) in
                self.underlying.ports().iter().filter(|(_, p)| p.is_sig())
            {
                let port_key = (None, idx.ul());
                let base = self.monosig.port_map[&port_key];
                self.monosig.port_data(
                    &self.underlying,
                    self.pass,
                    idx.ul(),
                    base,
                );
            }
        }

        // Handle event delays after monomorphization because delays might mention existential parameters.
        for (old, &new) in self.monosig.event_map.clone().iter() {
            self.monosig
                .event_delay(&self.underlying, self.pass, old, new);
        }

        self.monosig.base.take()
    }

    fn prop(&mut self, pidx: Underlying<ir::Prop>) -> Base<ir::Prop> {
        let prop = self.underlying.get(pidx);
        match self.underlying.get(pidx) {
            ir::Prop::True | ir::Prop::False => {
                self.monosig.base.add(prop.clone())
            }
            ir::Prop::Cmp(cmp) => {
                let ir::CmpOp { op, lhs, rhs } = cmp;
                let lhs = self.monosig.expr(&self.underlying, lhs.ul()).get();
                let rhs = self.monosig.expr(&self.underlying, rhs.ul()).get();
                self.monosig.base.add(ir::Prop::Cmp(ir::CmpOp {
                    op: op.clone(),
                    lhs,
                    rhs,
                }))
            }
            ir::Prop::TimeCmp(tcmp) => {
                let ir::CmpOp { op, lhs, rhs } = tcmp;
                let lhs = lhs.ul();
                let rhs = rhs.ul();
                let lhs = self.monosig.time(&self.underlying, self.pass, lhs);
                let rhs = self.monosig.time(&self.underlying, self.pass, rhs);
                self.monosig.base.add(ir::Prop::TimeCmp(ir::CmpOp {
                    op: op.clone(),
                    lhs: lhs.get(),
                    rhs: rhs.get(),
                }))
            }
            ir::Prop::TimeSubCmp(tscmp) => {
                let ir::CmpOp { op, lhs, rhs } = tscmp;
                let lhs =
                    self.monosig.timesub(&self.underlying, self.pass, lhs);
                let rhs =
                    self.monosig.timesub(&self.underlying, self.pass, rhs);
                self.monosig.base.add(ir::Prop::TimeSubCmp(ir::CmpOp {
                    op: op.clone(),
                    lhs,
                    rhs,
                }))
            }
            ir::Prop::Not(p) => {
                let p = p.ul();
                let new_p = self.prop(p);
                self.monosig.base.add(ir::Prop::Not(new_p.get()))
            }
            ir::Prop::And(l, r) => {
                let l = l.ul();
                let r = r.ul();
                let l = self.prop(l);
                let r = self.prop(r);
                self.monosig.base.add(ir::Prop::And(l.get(), r.get()))
            }
            ir::Prop::Or(l, r) => {
                let l = l.ul();
                let r = r.ul();
                let l = self.prop(l);
                let r = self.prop(r);
                self.monosig.base.add(ir::Prop::Or(l.get(), r.get()))
            }
            ir::Prop::Implies(l, r) => {
                let l = l.ul();
                let r = r.ul();
                let l = self.prop(l);
                let r = self.prop(r);
                self.monosig.base.add(ir::Prop::Implies(l.get(), r.get()))
            }
        }
    }

    fn access(&mut self, acc: &ir::Access) -> ir::Access {
        let ir::Access { port, ranges } = acc;

        let mut conc = |e: ir::ExprIdx| -> ir::ExprIdx {
            let base = self.monosig.expr(&self.underlying, e.ul());
            self.monosig
                .base
                .bin(self.monosig.base.get(base).clone())
                .get()
        };

        // generate end expression
        let ranges = ranges
            .iter()
            .map(|(s, e)| (conc(*s), conc(*e)))
            .collect_vec();

        let port = self.monosig.port_use(&self.underlying, port.ul()).get();

        ir::Access { port, ranges }
    }

    fn connect(&mut self, con: &ir::Connect) -> ir::Connect {
        let ir::Connect { src, dst, info } = con;

        let mono_src = self.access(src);
        let mono_dst = self.access(dst);
        let info = info.ul();

        ir::Connect {
            src: mono_src,
            dst: mono_dst,
            info: self.monosig.info(&self.underlying, self.pass, info).get(),
        }
    }

    fn forloop(&mut self, lp: &ir::Loop) {
        let ir::Loop {
            index,
            start,
            end,
            body,
        } = lp;

        let mono_start = self.monosig.expr(&self.underlying, start.ul()).get();
        let mono_end = self.monosig.expr(&self.underlying, end.ul()).get();

        let mut i = mono_start.as_concrete(self.monosig.base.comp()).unwrap();
        let bound = mono_end.as_concrete(self.monosig.base.comp()).unwrap();

        while i < bound {
            let index = index.ul();
            let orig_l = self.monosig.binding.len();
            self.monosig.binding.push(index, i);
            for cmd in body.iter() {
                let cmd = self.command(cmd);
                self.monosig.base.extend_cmds(cmd);
            }
            // Remove all the bindings added in this scope including the index
            self.monosig
                .binding
                .pop_n(self.monosig.binding.len() - orig_l);
            i += 1;
        }
    }

    fn if_stmt(&mut self, if_stmt: &ir::If) {
        let ir::If { cond, then, alt } = if_stmt;

        let cond = cond.ul();
        let cond = self.prop(cond);
        let cond = self
            .monosig
            .base
            .resolve_prop(self.monosig.base.get(cond).clone());

        let branch = match self.monosig.base.get(cond) {
            ir::Prop::True => then,
            ir::Prop::False => alt,
            cond => self
                .monosig
                .base
                .comp()
                .internal_error(format!("Non-bool condition: {cond}")),
        };

        let body = branch
            .iter()
            .map(|cmd| self.command(cmd))
            .fold(&mut vec![], |acc, cvec| {
                acc.extend(cvec);
                acc
            })
            .to_vec();

        self.monosig.base.extend_cmds(body);
    }

    /// Compile the given command and return the generated command if any.
    fn command(&mut self, cmd: &ir::Command) -> Option<ir::Command> {
        match cmd {
            ir::Command::Instance(idx) => Some(
                self.monosig
                    .inst_def(&self.underlying, self.pass, idx.ul())
                    .get()
                    .into(),
            ),
            ir::Command::Invoke(idx) => Some(
                self.monosig
                    .inv_def(&self.underlying, self.pass, idx.ul())
                    .get()
                    .into(),
            ),
            ir::Command::BundleDef(p) => Some(
                self.monosig
                    .local_port_def(&self.underlying, self.pass, p.ul())
                    .get()
                    .into(),
            ),
            ir::Command::Connect(con) => Some(self.connect(con).into()),
            ir::Command::ForLoop(lp) => {
                self.forloop(lp);
                None
            }
            ir::Command::If(if_stmt) => {
                self.if_stmt(if_stmt);
                None
            }
            ir::Command::Exists(ir::Exists { param, expr }) => {
                let comp_key = self.comp_key();
                let e = self.monosig.expr(&self.underlying, expr.ul()).get();
                let base_comp = self.monosig.base.comp();
                let Some(v) = e.as_concrete(base_comp) else {
                    unreachable!(
                        "exists binding evaluated to: {}",
                        base_comp.display(e)
                    )
                };

                self.pass
                    .inst_info_mut(comp_key)
                    .add_exist_val(param.ul(), v);
                None
            }
            // XXX(rachit): We completely get rid of facts in the program here.
            // If we want to do this long term, this should be done in a
            // separate pass and monomorphization should fail on facts.
            ir::Command::Fact(_) => None,
        }
    }
}
