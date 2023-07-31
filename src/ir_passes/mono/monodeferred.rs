use super::{monosig::MonoSig, Monomorphize};
use crate::ir::{self, Ctx};
use itertools::Itertools;

pub struct MonoDeferred<'a, 'pass: 'a> {
    /// The underlying component to be monomorphized
    pub underlying: &'a ir::Component,
    /// Underlying pointer
    pub pass: &'a mut Monomorphize<'pass>,
    /// Struct to keep track of all the mapping information from things owned by
    /// `underlying` to things owned by `base`
    pub monosig: MonoSig,
}

impl MonoDeferred<'_, '_> {
    pub fn sig(
        monosig: &mut MonoSig,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
    ) {
        // Events can be recursive, so do a pass over them to generate the new idxs now
        // and then fill them in later
        let binding = monosig.binding.inner();
        let conc_params = binding
            .iter()
            .filter(|(p, _)| underlying.get(*p).is_sig_owned())
            .map(|(_, n)| *n)
            .collect_vec();
        for (idx, event) in underlying.events().iter() {
            let new_idx = monosig.base.add(event.clone());
            monosig.event_map.insert(idx, new_idx);
            pass.event_map.insert(
                (monosig.underlying_idx, conc_params.clone(), idx),
                new_idx,
            );
        }

        if underlying.is_ext {
            for (_, param) in underlying.params().iter() {
                monosig.base.add(param.clone());
            }
        }

        for (idx, port) in underlying.ports().iter() {
            if port.is_sig_in() || port.is_sig_out() {
                monosig.port(underlying, pass, idx);
            }
        }

        for (old, new) in monosig.event_map.clone().iter() {
            monosig.event_second(underlying, pass, *old, *new);
        }

        monosig.interface(&underlying.src_info);
        monosig.base.unannotated_ports = underlying.unannotated_ports.clone();
    }
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    pub fn gen_comp(&mut self) {
        for cmd in &self.underlying.cmds {
            let cmd = self.command(cmd);
            self.monosig.base.cmds.extend(cmd);
        }
        for fact in self.monosig.fact_queue.clone() {
            if let Some(cmd) = self.fact(&fact) {
                self.monosig.base.cmds.push(cmd);
            }
        }
        for idx in self.underlying.props().idx_iter() {
            self.prop(idx);
        }
    }

    fn prop(&mut self, pidx: ir::PropIdx) -> ir::PropIdx {
        if let Some(idx) = self.monosig.prop_map.get(&pidx) {
            return *idx;
        };

        let prop = self.underlying.get(pidx);
        match self.underlying.get(pidx) {
            ir::Prop::True | ir::Prop::False => {
                let new_idx = self.monosig.base.add(prop.clone());
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::Cmp(cmp) => {
                let ir::CmpOp { op, lhs, rhs } = cmp;
                let lhs = self.monosig.expr(self.underlying, self.pass, *lhs);
                let rhs = self.monosig.expr(self.underlying, self.pass, *rhs);
                let new_idx = self.monosig.base.add(ir::Prop::Cmp(ir::CmpOp {
                    op: op.clone(),
                    lhs,
                    rhs,
                }));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::TimeCmp(tcmp) => {
                let ir::CmpOp { op, lhs, rhs } = tcmp;
                let lhs = self.monosig.time(self.underlying, self.pass, *lhs);
                let rhs = self.monosig.time(self.underlying, self.pass, *rhs);
                let new_idx =
                    self.monosig.base.add(ir::Prop::TimeCmp(ir::CmpOp {
                        op: op.clone(),
                        lhs,
                        rhs,
                    }));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::TimeSubCmp(tscmp) => {
                let ir::CmpOp { op, lhs, rhs } = tscmp;
                let lhs = self.monosig.timesub(self.underlying, self.pass, lhs);
                let rhs = self.monosig.timesub(self.underlying, self.pass, rhs);
                let new_idx =
                    self.monosig.base.add(ir::Prop::TimeSubCmp(ir::CmpOp {
                        op: op.clone(),
                        lhs,
                        rhs,
                    }));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::Not(p) => {
                let new_p = self.prop(*p);
                let new_idx = self.monosig.base.add(ir::Prop::Not(new_p));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::And(l, r) => {
                let l = self.prop(*l);
                let r = self.prop(*r);
                let new_idx = self.monosig.base.add(ir::Prop::And(l, r));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::Or(l, r) => {
                let l = self.prop(*l);
                let r = self.prop(*r);
                let new_idx = self.monosig.base.add(ir::Prop::Or(l, r));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
            ir::Prop::Implies(l, r) => {
                let l = self.prop(*l);
                let r = self.prop(*r);
                let new_idx = self.monosig.base.add(ir::Prop::Implies(l, r));
                self.monosig.prop_map.insert(pidx, new_idx);
                new_idx
            }
        }
    }

    fn access(&mut self, acc: &ir::Access) -> ir::Access {
        let ir::Access { port, start, end } = acc;

        let port = self.monosig.port(self.underlying, self.pass, *port);

        // generate end expression
        let end = self.monosig.expr(self.underlying, self.pass, *end);
        // convert to concrete value
        let end = self.monosig.base.bin(self.monosig.base.get(end).clone());
        // generate start expression
        let start = self.monosig.expr(self.underlying, self.pass, *start);
        // convert to concrete value
        let start = self.monosig.base.bin(self.monosig.base.get(start).clone());

        ir::Access { port, start, end }
    }

    fn connect(&mut self, con: &ir::Connect) -> ir::Connect {
        let ir::Connect { src, dst, info } = con;

        let mono_src = self.access(src);
        let mono_dst = self.access(dst);

        ir::Connect {
            src: mono_src,
            dst: mono_dst,
            info: self.monosig.info(self.underlying, self.pass, info),
        }
    }

    fn forloop(&mut self, lp: &ir::Loop) {
        let ir::Loop {
            index,
            start,
            end,
            body,
        } = lp;

        // let mono_index = self.monosig.param(self.underlying, self.pass, *index);
        let mono_start = self.monosig.expr(self.underlying, self.pass, *start);
        let mono_end = self.monosig.expr(self.underlying, self.pass, *end);

        let mut i = mono_start.as_concrete(&self.monosig.base).unwrap();
        let bound = mono_end.as_concrete(&self.monosig.base).unwrap();

        while i < bound {
            self.monosig.handled_instances.clear();
            self.monosig.handled_invokes.clear();
            self.monosig.binding.insert(*index, i);
            for cmd in body.iter() {
                let cmd = self.command(cmd);
                self.monosig.base.cmds.extend(cmd);
            }
            self.monosig.binding.pop();
            i += 1;
        }
    }

    fn if_stmt(&mut self, if_stmt: &ir::If) {
        let ir::If { cond, then, alt } = if_stmt;

        let cond = self.prop(*cond);
        let cond = self
            .monosig
            .base
            .resolve_prop(self.monosig.base.get(cond).clone());

        let then = then
            .iter()
            .map(|cmd| self.command(cmd))
            .fold(&mut vec![], |acc, cvec| {
                acc.extend(cvec);
                acc
            })
            .to_vec();
        let alt = alt
            .iter()
            .map(|cmd| self.command(cmd))
            .fold(&mut vec![], |acc, cvec| {
                acc.extend(cvec);
                acc
            })
            .to_vec();

        match self.monosig.base.get(cond) {
            ir::Prop::True => self.monosig.base.cmds.extend(then),
            ir::Prop::False => self.monosig.base.cmds.extend(alt),
            _ => self
                .monosig
                .base
                .internal_error(format!("couldn't resolve {}", cond)),
        }
    }

    fn fact(&mut self, fact: &ir::Fact) -> Option<ir::Command> {
        let ir::Fact { prop, reason, .. } = fact;
        let prop = self.prop(*prop);
        let reason = self.monosig.info(self.underlying, self.pass, reason);
        if fact.is_assert() {
            self.monosig.base.assert(prop, reason)
        } else {
            self.monosig.base.assume(prop, reason)
        }
    }

    fn command(&mut self, cmd: &ir::Command) -> Vec<ir::Command> {
        match cmd {
            ir::Command::Instance(idx) => {
                if self.monosig.handled_instances.contains(idx) {
                    let inst_occurrences =
                        self.monosig.inst_counter.get(idx).unwrap();
                    let base_inst = *self
                        .monosig
                        .inst_map
                        .get(&(*idx, *inst_occurrences))
                        .unwrap();
                    vec![base_inst.into()]
                } else {
                    vec![self
                        .monosig
                        .instance(self.underlying, self.pass, *idx)
                        .into()]
                }
            }
            ir::Command::Invoke(idx) => {
                if self.monosig.handled_invokes.contains(idx) {
                    let inv_occurrences =
                        self.monosig.inv_counter.get(idx).unwrap();
                    let base_inv = *self
                        .monosig
                        .inv_map
                        .get(&(*idx, *inv_occurrences))
                        .unwrap();
                    vec![base_inv.into()]
                } else {
                    vec![self
                        .monosig
                        .invoke(self.underlying, self.pass, *idx)
                        .into()]
                }
            }
            ir::Command::Connect(con) => vec![self.connect(con).into()],
            ir::Command::ForLoop(lp) => {
                self.forloop(lp);
                vec![]
            }
            ir::Command::If(if_stmt) => {
                self.if_stmt(if_stmt);
                vec![]
            }
            ir::Command::Fact(fact) => {
                self.monosig.fact_queue.push(fact.clone());
                vec![]
            }
        }
    }
}
