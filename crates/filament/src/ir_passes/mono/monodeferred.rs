use crate::ir_passes::{mono::Concretize, schedule};

use super::{
    CompKey, IntoBase, IntoUdl, MonoSig, Monomorphize, Underlying,
    UnderlyingComp,
};
use fil_ir::{self as ir, AddCtx, Ctx, Foldable};
use fil_utils::{self as utils, AttrCtx};
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

    /// If this component should be scheduled, this stores the scheduling type
    pub schedule: bool,
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    pub fn new(
        underlying: UnderlyingComp<'a>,
        pass: &'a mut Monomorphize<'pass>,
        monosig: MonoSig,
    ) -> Self {
        Self {
            schedule: underlying.attrs().has(utils::CompNum::Schedule),
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

    /// Monomorphize the parts of the signature that do not use any existential parameters.
    /// We must call [Self::sig_complete_mono] at some point after calling this.
    /// Otherwise, the IR will be in an invalid state.
    pub fn sig_partial_mono(&mut self) {
        let comp_k: CompKey = self.monosig.comp_key.clone();

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

        // Do the second concretization pass
        let mut conc = Concretize::new(self.monosig.base.comp_mut());
        conc.comp();
        // Convert to an expression binding
        let binding = ir::Bind::new(
            conc.take()
                .iter()
                .copied()
                .map(|(b, v)| (b, self.monosig.base.num(v))),
        );
        // resolve bindings in the monosig binding with these new values

        // Insert the existential parameters into the monomorphization
        // Also extend the binding with existential parameters
        let exist_params = self
            .monosig
            .binding
            .iter()
            .filter(|(p, _)| self.underlying.get(*p).is_existential())
            .map(|(p, e)| {
                // Substitute in the outputted binding for the existential
                (
                    *p,
                    e.get()
                        .fold_with(self.monosig.base.comp_mut(), &mut |k| {
                            binding.get(&k).map(|v| v.get())
                        })
                        .concrete(self.monosig.base.comp()),
                )
            })
            .collect_vec();

        log::debug!(
            "{:?}",
            exist_params
                .iter()
                .map(|(p, v)| { (self.underlying.display(*p), v) })
                .collect_vec()
        );

        for (param, v) in exist_params {
            let comp_key = self.monosig.comp_key.clone();

            self.pass.inst_info_mut(comp_key).add_exist_val(param, v);

            // Add the concrete value to the binding to monomorphize later propositions
            self.push_binding(param, v);
        }

        // Make sure all assertions generated by existential parameters evaluate to true
        // The type checker ensures this cannot happen for Filament-level
        // components but externally generated components might violate their
        // requirements.
        for (prop, _) in self.underlying.all_exist_assumes() {
            let out = self
                .monosig
                .prop(&self.underlying, prop.ul(), self.pass)
                .unwrap();
            let Some(v) = out.get().as_concrete(self.monosig.base.comp())
            else {
                unreachable!(
                    "Failed to concretize proposition: {}",
                    self.underlying.display(prop.ul())
                )
            };
            assert!(
                v,
                "Existential parameter assertion failed: {}",
                self.underlying.display(prop.ul())
            );
        }

        // Mark the signature monormophization as complete
        self.sig_mono_complete = true;
    }

    /// Add to the parameter binding
    pub fn push_binding(&mut self, p: Underlying<ir::Param>, v: u64) {
        self.monosig.push_binding(p, v);
    }

    /// Monomorphize a component definition
    pub fn comp(mut self) -> ir::Component {
        assert!(!self.underlying.is_ext(), "cannot monomorphize external");

        // Monomorphize the part of the signature that doesn't use existential parameters
        self.sig_partial_mono();

        // Monomorphize the component's body
        self.body();

        // Run scheduling pass if needed
        if self.schedule {
            schedule::schedule(
                &self.pass.ctx,
                self.monosig.base.comp_mut(),
                self.pass.scheduling_reg,
                self.pass.opts.solver_replay_file.as_ref(),
            );
        }

        // Monomorphize the rest of the signature
        self.sig_complete_mono();

        let mut comp = self.monosig.base.take();

        // Remove all the let bindings that were added to this component
        comp.cmds.retain(|cmd| !matches!(cmd, ir::Command::Let(_)));

        // Return the component
        comp
    }

    fn access(&mut self, acc: &ir::Access) -> ir::Access {
        let ir::Access { port, ranges } = acc;

        let mut conc = |e: ir::ExprIdx| -> ir::ExprIdx {
            let base = self
                .monosig
                .expr(&self.underlying, e.ul(), self.pass)
                .unwrap();
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

        let mono_start = self
            .monosig
            .expr(&self.underlying, start.ul(), self.pass)
            .unwrap()
            .get();
        let mono_end = self
            .monosig
            .expr(&self.underlying, end.ul(), self.pass)
            .unwrap()
            .get();

        let mut i = mono_start.as_concrete(self.monosig.base.comp()).unwrap();
        let bound = mono_end.as_concrete(self.monosig.base.comp()).unwrap();

        while i < bound {
            let index = index.ul();
            let orig_l = self.monosig.binding.len();
            self.push_binding(index, i);
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
        let cond = self
            .monosig
            .prop(&self.underlying, cond, self.pass)
            .unwrap();
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

        if let Ok(prop) = self.monosig.prop(&self.underlying, prop, self.pass) {
            let reason = reason.ul();
            let reason =
                self.monosig.info(&self.underlying, self.pass, reason).get();

            // After monomorphization, both assumes and asserts become asserts.
            Some(ir::Fact::assert(prop.get(), reason))
        } else {
            // This prop is not valid. This can be caused by assertions that are out of the
            // scope of where parameters are bound, and will occur if the the definition is
            // in a branch that is not taken.
            None
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
            ir::Command::Let(ir::Let { param, expr }) => {
                let p = param.ul();

                match expr {
                    ir::MaybeUnknown::Known(expr) => {
                        let e = self
                            .monosig
                            .expr(&self.underlying, expr.ul(), self.pass)
                            .unwrap()
                            .get();
                        self.monosig.binding.push(p, e.base());
                        None
                    }
                    ir::MaybeUnknown::Unknown(params) => {
                        if !self.schedule {
                            unreachable!(
                                "Encountered `?` let binding in non-scheduled component"
                            );
                        }

                        let bind = ir::MaybeUnknown::Unknown(
                            params
                                .iter()
                                .map(|p| {
                                    self.monosig
                                        .param_use(&self.underlying, p.ul())
                                        .unwrap()
                                        .get()
                                })
                                .collect(),
                        );

                        let new_param = self.monosig.unelaborated_param(
                            &self.underlying,
                            self.pass,
                            p,
                            ir::ParamOwner::Let { bind: bind.clone() },
                        );

                        Some(
                            ir::Let {
                                param: new_param.get(),
                                expr: bind,
                            }
                            .into(),
                        )
                    }
                }
            }
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
                let expr = self
                    .monosig
                    .expr(&self.underlying, expr.ul(), self.pass)
                    .unwrap();

                self.monosig.binding.push(param.ul(), expr);
                None
            }
            // XXX(rachit): We completely get rid of facts in the program here.
            // If we want to do this long term, this should be done in a
            // separate pass and monomorphization should fail on facts.
            ir::Command::Fact(fact) => self.fact(fact).map(|f| f.into()),
        }
    }

    pub fn body(&mut self) {
        for cmd in self.underlying.cmds().clone() {
            let cmd = self.command(&cmd);
            self.monosig.base.extend_cmds(cmd);
        }

        // Monomorphize the ports in the signature that are not already monomorphized
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
    }
}
