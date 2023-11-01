use super::{
    Base, CompKey, IntoUdl, MonoSig, Monomorphize, Underlying, UnderlyingComp,
};
use fil_ir::{self as ir, AddCtx, Ctx};
use ir::DisplayCtx;
use itertools::Itertools;

/// Defines methods required to monomorphize a component. Most of the updates
/// happen to the stored [MonoSig] object which keeps all the maps needed while
/// monomorphizing the component.
pub struct MonoDeferred<'comp, 'pass: 'comp> {
    /// The underlying component to be monomorphized
    pub underlying: UnderlyingComp<'comp>,
    /// Underlying pointer
    pub pass: &'comp mut Monomorphize<'pass>,
    /// Struct to keep track of all the mapping information from things owned by
    /// `underlying` to things owned by `base`
    pub monosig: MonoSig,

    /// Have we completed the monomorphization of the signature?
    sig_mono_complete: bool,
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    pub fn new(
        underlying: UnderlyingComp<'a>,
        pass: &'a mut Monomorphize<'pass>,
        monosig: MonoSig,
    ) -> Self {
        Self {
            underlying,
            pass,
            monosig,
            // Set to false if we call `sig_partial_mono` and true once we call `sig_complete_mono`
            sig_mono_complete: true,
        }
    }
}

impl MonoDeferred<'_, '_> {
    /// Get the underlying component
    pub fn take(self) -> ir::Component {
        self.monosig.base.take()
    }

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

    /// Monomorphize the parts of the signature that do not use any existential parameters.
    /// We must call [Self::sig_complete_mono] at some point after calling this.
    /// Otherwise, the IR will be in an invalid state.
    pub fn sig_partial_mono(&mut self) {
        let comp_k: CompKey = self.comp_key();

        let monosig = &mut self.monosig;
        let ul = &self.underlying;
        let pass = &mut self.pass;

        for (idx, event) in ul.events().iter() {
            // Add events without monomorphizing their data. The data is updated after the body is monomorphized
            // because it can mention existential parameters.
            let new_idx = monosig.base.add(event.clone());
            monosig.event_map.insert(idx.ul(), new_idx);
            pass.inst_info_mut(comp_k.clone())
                .add_event(idx.ul(), new_idx);
        }

        // Monomorphize port in the signature. Other ports are monomorphized
        // while traversing commands.
        for (idx, port) in ul.ports().iter() {
            if port.is_sig() {
                // Only monomorphize the definition without the data.
                // The data is monomorphized after the body so that we have bindings for existential parameters.
                let port = monosig.port_def_partial(ul, pass, idx.ul());
                // Add these ports to the global port information for this
                // instance because other components need this information
                // to resolve foreigns.
                pass.inst_info_mut(comp_k.clone()).add_port(idx.ul(), port)
            }
        }

        let src_info = ul.src_info();
        monosig.interface(src_info);

        let unannotated_ports = ul.unannotated_ports().clone();
        monosig.base.set_unannotated_ports(unannotated_ports);

        // Mark the signature monormophization as incomplete
        self.sig_mono_complete = false;
    }

    /// Monomorphize the parts of the signature that use existential parameters.
    pub fn sig_complete_mono(&mut self) {
        assert!(!self.sig_mono_complete);

        for (idx, _) in
            self.underlying.ports().iter().filter(|(_, p)| p.is_sig())
        {
            let port_key = (None, idx.ul());
            let base = self.monosig.port_map[&port_key];
            self.monosig
                .port_data(&self.underlying, self.pass, idx.ul(), base);
        }

        // Handle event delays after monomorphization because delays might mention existential parameters.
        for (old, &new) in self.monosig.event_map.clone().iter() {
            self.monosig
                .event_delay(&self.underlying, self.pass, old, new);
        }

        // Mark the signature monormophization as complete
        self.sig_mono_complete = true;
    }

    /// Add to the parameter binding
    pub fn push_binding(&mut self, p: Underlying<ir::Param>, v: u64) {
        self.monosig.binding.push(p, v);
    }

    /// Monomorphize a component definition
    pub fn comp(mut self) -> ir::Component {
        assert!(!self.underlying.is_ext(), "cannot monomorphize external");

        // Monomorphize the part of the signature that doesn't use existential parameters
        self.sig_partial_mono();

        // Monomorphize the component's body
        for cmd in self.underlying.cmds().clone() {
            let cmd = self.command(&cmd);
            self.monosig.base.extend_cmds(cmd);
        }

        // Extend the binding with existential parameters
        let info = self.pass.inst_info(&self.comp_key());
        for param in self.underlying.exist_params() {
            let param = param.ul();
            let Some(v) = info.get_exist_val(param) else {
                unreachable!(
                        "No binding for existential parameter `{}'. Is the body missing an assignment to the parameter?",
                        self.underlying.display(param)
                    )
            };
            self.monosig.binding.push(param, v);
        }

        // Monomorphize the rest of the signature
        self.sig_complete_mono();

        // Return the component
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
                // concretize the left expression first because if it is false the right might be invalid
                // For example, `(X > 0) & (X - 1 >= 0)` would fail due to overflow.
                match self.monosig.base.get(l) {
                    fil_ir::Prop::True => self.prop(r),
                    fil_ir::Prop::False => {
                        self.monosig.base.add(ir::Prop::True)
                    }
                    _ => {
                        let r = self.prop(r);
                        self.monosig
                            .base
                            .add(ir::Prop::Implies(l.get(), r.get()))
                    }
                }
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

    fn fact(&mut self, fact: &ir::Fact) -> Option<ir::Fact> {
        let ir::Fact { prop, reason, .. } = fact;

        let prop = prop.ul();
        log::debug!("Fact: {}", self.underlying.display(prop));
        let (params, _) = self.underlying.relevant_vars(prop);

        if !params
            .into_iter()
            .all(|idx| self.monosig.binding.get(&idx).is_some())
        {
            // Discards the assertion if it references parameters that can't be resolved (bundle parameters, etc)
            // TODO(edmund): Find a better solution to this - we should resolve bundle assertions when bundles are unrolled.
            None
        } else {
            log::debug!(
                "Resolving: {} with binding {}",
                self.underlying.display(prop),
                self.monosig.binding_rep(&self.underlying)
            );
            let prop = self.prop(prop);
            log::debug!("Resolved: {}", self.monosig.base.display(prop));
            let prop = self
                .monosig
                .base
                .resolve_prop(self.monosig.base.get(prop).clone())
                .get();

            let reason = reason.ul();
            let reason =
                self.monosig.info(&self.underlying, self.pass, reason).get();

            // After monomorphization, both assumes and asserts become asserts.
            Some(ir::Fact::assert(prop, reason))
        }
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
            ir::Command::Fact(fact) => self.fact(fact).map(|f| f.into()),
        }
    }
}
