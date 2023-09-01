use super::{
    monosig::MonoSig,
    utils::{Base, Underlying, UnderlyingComp},
    Monomorphize,
};
use crate::ir::{self, AddCtx, Ctx};
use itertools::Itertools;

pub(super) struct MonoDeferred<'a, 'pass: 'a> {
    /// The underlying component to be monomorphized
    pub underlying: UnderlyingComp<'a>,
    /// Underlying pointer
    pub pass: &'a mut Monomorphize<'pass>,
    /// Struct to keep track of all the mapping information from things owned by
    /// `underlying` to things owned by `base`
    pub monosig: MonoSig,
}

impl MonoDeferred<'_, '_> {
    // XXX(rachit): Why does this function need to do anything to the signature
    // of external components instead of just wholesale copying them?
    pub fn sig(
        monosig: &mut MonoSig,
        underlying: UnderlyingComp,
        pass: &mut Monomorphize,
    ) {
        let binding = monosig.binding.inner();
        let conc_params = if underlying.is_ext() {
            vec![]
        } else {
            binding
                .iter()
                .filter(|(p, _)| underlying.get(*p).is_sig_owned() || underlying.get(*p).is_sig_bind())
                .map(|(_, n)| *n)
                .collect_vec()
        };
        // Events can be recursive, so do a pass over them to generate the new idxs now
        // and then fill them in later
        for (idx, event) in underlying.events().iter() {
            let new_idx = monosig.base.add(event.clone());
            monosig.event_map.insert(Underlying::new(idx), new_idx);
            pass.event_map.insert(
                (
                    (
                        Underlying::new(monosig.underlying_idx),
                        conc_params.clone(),
                    )
                        .into(),
                    Underlying::new(idx),
                ),
                new_idx,
            );
        }

        if underlying.is_ext() {
            // We can copy over the underlying expressions because we're not
            // going to substitute anything.
            for (_, expr) in underlying.exprs().iter() {
                monosig.base.add(expr.clone());
            }

            // Add all parameters because we're not going to substitute them
            for (idx, param) in underlying.params().iter() {
                let ir::Param { owner, info } = param;
                let info = Underlying::new(*info);
                let param = ir::Param {
                    owner: owner.clone(),
                    info: monosig.info(&underlying, pass, info).get(),
                };
                let new_idx = monosig.base.add(param);
                monosig.param_map.insert(Underlying::new(idx), new_idx);
            }

            for (idx, port) in underlying.ports().iter() {
                if port.is_sig() {
                    monosig.ext_port(&underlying, pass, Underlying::new(idx));
                }
            }
        } else {
            for (param, expr) in underlying.sig_binding() {
                let new_expr =
                    monosig.expr(&underlying, Underlying::new(*expr));
                let new_expr = monosig
                    .base
                    .bin(monosig.base.get(new_expr).clone())
                    .get()
                    .as_concrete(monosig.base.comp())
                    .unwrap();
                monosig.binding.insert(Underlying::new(*param), new_expr);
            }

            for (idx, port) in underlying.ports().iter() {
                if port.is_sig() {
                    let port = monosig.port_def(
                        &underlying,
                        pass,
                        Underlying::new(idx),
                    );
                    pass.port_map.insert(
                        (
                            (
                                Underlying::new(monosig.underlying_idx),
                                conc_params.clone(),
                            )
                                .into(),
                            Underlying::new(idx),
                        ),
                        port,
                    );
                }
            }
        }

        for (old, new) in monosig.event_map.clone().iter() {
            monosig.event_second(&underlying, pass, old, *new);
        }

        let src_info = underlying.src_info();
        let unannotated_ports = underlying.unannotated_ports().clone();

        monosig.interface(&underlying, src_info);
        monosig.base.set_unannotated_ports(unannotated_ports);
    }
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    pub fn gen_comp(&mut self) {
        // is there a way to not clone this?
        for cmd in self.underlying.cmds().clone() {
            let cmd = self.command(&cmd);
            self.monosig.base.extend_one(cmd);
        }
    }

    fn prop(&mut self, pidx: Underlying<ir::Prop>) -> Base<ir::Prop> {
        let prop = self.underlying.get(pidx);
        match self.underlying.get(pidx) {
            ir::Prop::True | ir::Prop::False => {
                self.monosig.base.add(prop.clone())
            }
            ir::Prop::Cmp(cmp) => {
                let ir::CmpOp { op, lhs, rhs } = cmp;
                let lhs = self
                    .monosig
                    .expr(&self.underlying, Underlying::new(*lhs))
                    .get();
                let rhs = self
                    .monosig
                    .expr(&self.underlying, Underlying::new(*rhs))
                    .get();
                self.monosig.base.add(ir::Prop::Cmp(ir::CmpOp {
                    op: op.clone(),
                    lhs,
                    rhs,
                }))
            }
            ir::Prop::TimeCmp(tcmp) => {
                let ir::CmpOp { op, lhs, rhs } = tcmp;
                let lhs = Underlying::new(*lhs);
                let rhs = Underlying::new(*rhs);
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
                let p = Underlying::new(*p);
                let new_p = self.prop(p);
                self.monosig.base.add(ir::Prop::Not(new_p.get()))
            }
            ir::Prop::And(l, r) => {
                let l = Underlying::new(*l);
                let r = Underlying::new(*r);
                let l = self.prop(l);
                let r = self.prop(r);
                self.monosig.base.add(ir::Prop::And(l.get(), r.get()))
            }
            ir::Prop::Or(l, r) => {
                let l = Underlying::new(*l);
                let r = Underlying::new(*r);
                let l = self.prop(l);
                let r = self.prop(r);
                self.monosig.base.add(ir::Prop::Or(l.get(), r.get()))
            }
            ir::Prop::Implies(l, r) => {
                let l = Underlying::new(*l);
                let r = Underlying::new(*r);
                let l = self.prop(l);
                let r = self.prop(r);
                self.monosig.base.add(ir::Prop::Implies(l.get(), r.get()))
            }
        }
    }

    fn access(&mut self, acc: &ir::Access) -> ir::Access {
        let ir::Access { port, start, end } = acc;

        let port = self
            .monosig
            .port_use(&self.underlying, Underlying::new(*port))
            .get();

        // generate end expression
        let end = self.monosig.expr(&self.underlying, Underlying::new(*end));

        // convert to concrete value
        let end = self.monosig.base.bin(self.monosig.base.get(end).clone());

        // generate start expression
        let start =
            self.monosig.expr(&self.underlying, Underlying::new(*start));

        // convert to concrete value
        let start = self.monosig.base.bin(self.monosig.base.get(start).clone());

        ir::Access {
            port,
            start: start.get(),
            end: end.get(),
        }
    }

    fn connect(&mut self, con: &ir::Connect) -> ir::Connect {
        let ir::Connect { src, dst, info } = con;

        let mono_src = self.access(src);
        let mono_dst = self.access(dst);
        let info = Underlying::new(*info);

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
            .expr(&self.underlying, Underlying::new(*start))
            .get();
        let mono_end = self
            .monosig
            .expr(&self.underlying, Underlying::new(*end))
            .get();

        let mut i = mono_start.as_concrete(self.monosig.base.comp()).unwrap();
        let bound = mono_end.as_concrete(self.monosig.base.comp()).unwrap();

        while i < bound {
            let index = Underlying::new(*index);
            self.monosig.binding.insert(index, i);
            for cmd in body.iter() {
                let cmd = self.command(cmd);
                self.monosig.base.extend_one(cmd);
            }
            // Pop all the let bindings
            self.monosig.binding.pop(); // pop the index
            i += 1;
        }
    }

    fn if_stmt(&mut self, if_stmt: &ir::If) {
        let ir::If { cond, then, alt } = if_stmt;

        let cond = Underlying::new(*cond);
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
                    .inst_def(
                        &self.underlying,
                        self.pass,
                        Underlying::new(*idx),
                    )
                    .get()
                    .into(),
            ),
            ir::Command::Invoke(idx) => Some(
                self.monosig
                    .inv_def(&self.underlying, self.pass, Underlying::new(*idx))
                    .get()
                    .into(),
            ),
            ir::Command::BundleDef(p) => Some(
                self.monosig
                    .port_def(&self.underlying, self.pass, Underlying::new(*p))
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
            // XXX(rachit): We completely get rid of facts in the program here.
            // If we want to do this long term, this should be done in a
            // separate pass and monomorphization should fail on facts.
            ir::Command::Fact(_) => None,
        }
    }
}
