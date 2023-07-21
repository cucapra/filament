use super::{monosig::MonoSig, Monomorphize};
use crate::ir::{self, Ctx, Foreign, MutCtx};
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
        println!("processing the sig of {}", monosig.underlying_idx);
        for idx in underlying.events().idx_iter() {
            monosig.event(underlying, pass, idx);
        }
        for (idx, port) in underlying.ports().iter() {
            if port.is_sig_in() || port.is_sig_out() {
                monosig.port(underlying, pass, idx);
            }
        }
    }
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    pub fn gen_comp(&mut self) {
        println!("generating a new comp for {}", self.monosig.underlying_idx);
        for cmd in &self.underlying.cmds {
            let cmd = self.command(cmd);
            self.monosig.base.cmds.extend(cmd);
        }
        for idx in self.underlying.props().idx_iter() {
            self.prop(idx);
        }
    }

    fn foreign_event(
        &mut self,
        foreign: &Foreign<ir::Event, ir::Component>,
        inv: ir::InvIdx, // underlying
    ) -> Foreign<ir::Event, ir::Component> {
        let Foreign { key, .. } = foreign;
        // `key` is only meaningful in `owner`
        // need to map `key` to be the monomorphized index and update `owner` to be
        // the monomorphized component

        let inst = self.underlying.get(self.underlying.get(inv).inst);
        let inst_comp = inst.comp;
        let inst_params = &inst.params;
        let conc_params = inst_params
            .iter()
            .map(|p| {
                self.monosig
                    .expr(self.underlying, *p)
                    .as_concrete(&self.monosig.base)
                    .unwrap()
            })
            .collect_vec();

        let conc_params_copy = conc_params.clone();

        println!(
            "looking up ({}, {:?}, {})",
            inst_comp, conc_params_copy, key
        );
        let new_event = self
            .pass
            .event_map
            .get(&(inst_comp, conc_params, *key))
            .unwrap();
        println!("got {}", new_event);

        let new_owner = if let Some((mono_compidx, _)) =
            self.pass.queue.get(&(inst_comp, conc_params_copy)) {
                *mono_compidx
            } else {
                self.monosig.underlying_idx
            };

        ir::Foreign {
            key: *new_event,
            owner: new_owner,
        }
    }

    /// Monomorphize the `inst` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn instance(&mut self, inst: ir::InstIdx) -> ir::InstIdx {
        // Count another time we've seen the instance
        self.insert_inst(inst);

        let ir::Instance { comp, params, info } = self.underlying.get(inst);
        let conc_params = params
            .iter()
            .map(|p| {
                self.monosig
                    .expr(self.underlying, *p)
                    .as_concrete(&self.monosig.base)
                    .unwrap()
            })
            .collect_vec();
        let (comp, params) = self.pass.should_process(*comp, conc_params);
        let new_inst = ir::Instance {
            comp,
            params: params
                .into_iter()
                .map(|n| self.monosig.base.num(n))
                .collect(),
            info: self.monosig.info(self.underlying, info),
        };

        let new_idx = self.monosig.base.add(new_inst);

        let inst_occurrences = self.monosig.inst_counter.get(&inst).unwrap();
        self.monosig
            .inst_map
            .insert((inst, *inst_occurrences), new_idx);
        new_idx
    }

    fn timesub(&mut self, timesub: &ir::TimeSub) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(expr) => {
                ir::TimeSub::Unit(self.monosig.expr(self.underlying, *expr))
            }
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.monosig.time(self.underlying, self.pass, *l),
                r: self.monosig.time(self.underlying, self.pass, *r),
            },
        }
    }
    /// Monomorphize the `inv` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn invoke(&mut self, inv: ir::InvIdx) -> ir::InvIdx {
        // Count another time that we've seen inv
        self.insert_inv(inv);

        // Need to monomorphize all parts of the invoke
        let ir::Invoke {
            inst,
            ports,
            events,
            info,
        } = self.underlying.get(inv);

        let info = self.monosig.info(self.underlying, info);

        // PLACEHOLDER, just want the index when we add it to base
        let mono_inv_idx = self.monosig.base.add(ir::Invoke {
            inst: *inst,
            ports: ports.clone(),
            events: events.clone(),
            info,
        });

        // Update the mapping from underlying invokes to base invokes
        // just unwrap because we maintain that inv will always be present in the mapping
        let inv_occurrences = self.monosig.inv_counter.get(&inv).unwrap();
        self.monosig
            .inv_map
            .insert((inv, *inv_occurrences), mono_inv_idx);

        // Handle connects we just saw:
        let connects = self.monosig.connects.clone();
        for con in connects.iter() {
            let cmd = self.connect(con);
            self.monosig.base.cmds.push(cmd.into());
        }
        self.monosig.connects.clear();

        // Instance - replace the instance owned by self.underlying with one owned by self.base
        let inst_occurrences = self.monosig.inst_counter.get(inst).unwrap();
        let base_inst = *self
            .monosig
            .inst_map
            .get(&(*inst, *inst_occurrences))
            .unwrap();

        // Ports
        let mono_ports = ports
            .iter()
            .map(|p| self.monosig.port(self.underlying, self.pass, *p))
            .collect_vec();

        // Events
        let mono_events =
            events.iter().map(|e| self.eventbind(e, inv)).collect_vec();

        // Build the new invoke, add it to self.base
        let mut mono_inv = self.monosig.base.get_mut(mono_inv_idx);

        mono_inv.inst = base_inst;
        mono_inv.ports = mono_ports;
        mono_inv.events = mono_events;

        mono_inv_idx
    }

    /// Update the mapping of how many times we've seen each invoke in the underlying component.
    /// If the given invoke does not exist in the mapping, add it with a counter of 0
    /// If it does exist, increment the counter by 1
    fn insert_inv(&mut self, inv: ir::InvIdx) {
        if let Some(n) = self.monosig.inv_counter.get(&inv) {
            self.monosig.inv_counter.insert(inv, *n + 1);
        } else {
            self.monosig.inv_counter.insert(inv, 0);
        }
    }

    fn insert_inst(&mut self, inst: ir::InstIdx) {
        if let Some(n) = self.monosig.inst_counter.get(&inst) {
            self.monosig.inst_counter.insert(inst, *n + 1);
        } else {
            self.monosig.inst_counter.insert(inst, 0);
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
                let lhs = self.monosig.expr(self.underlying, *lhs);
                let rhs = self.monosig.expr(self.underlying, *rhs);
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
                let lhs = self.timesub(lhs);
                let rhs = self.timesub(rhs);
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
        let start = self.monosig.expr(self.underlying, *start);
        let end = self.monosig.expr(self.underlying, *end);

        ir::Access { port, start, end }
    }

    fn connect(&mut self, con: &ir::Connect) -> ir::Connect {
        let ir::Connect { src, dst, info } = con;

        let mono_src = self.access(src);
        let mono_dst = self.access(dst);

        ir::Connect {
            src: mono_src,
            dst: mono_dst,
            info: self.monosig.info(self.underlying, info),
        }
    }

    fn forloop(&mut self, lp: &ir::Loop) {
        let ir::Loop {
            index,
            start,
            end,
            body,
        } = lp;

        //let mono_index = self.param(*index);
        let mono_start = self.monosig.expr(self.underlying, *start);
        let mono_end = self.monosig.expr(self.underlying, *end);
        // let mono_body = body
        //     .iter()
        //     .map(|cmd| self.command(cmd))
        //     .fold(&mut vec![], |acc, cvec| { acc.extend(cvec); acc})
        //     .to_vec();

        let mut i = mono_start.as_concrete(&self.monosig.base).unwrap();
        let bound = mono_end.as_concrete(&self.monosig.base).unwrap();

        while i < bound {
            self.monosig.binding.insert(*index, i);
            for cmd in body.iter() {
                let cmd = self.command(cmd);
                self.monosig.base.cmds.extend(cmd);
            }
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

        match cond {
            ir::Prop::True => self.monosig.base.cmds.extend(then),
            ir::Prop::False => self.monosig.base.cmds.extend(alt),
            _ => self.monosig.base.internal_error(format!("couldn't resolve {}", cond)),
        }
    }

    fn fact(&mut self, fact: &ir::Fact) -> Option<ir::Command> {
        let ir::Fact { prop, reason, .. } = fact;

        let prop = self.prop(*prop);
        let reason = self.monosig.info(self.underlying, reason);
        if fact.is_assert() {
            self.monosig.base.assert(prop, reason)
        } else {
            self.monosig.base.assume(prop, reason)
        }
    }

    fn eventbind(
        &mut self,
        eb: &ir::EventBind,
        inv: ir::InvIdx,
    ) -> ir::EventBind {
        let ir::EventBind {
            arg,
            info,
            delay,
            base,
        } = eb;

        let base = self.foreign_event(base, inv);
        let delay = self.timesub(delay);
        let arg = self.monosig.time(self.underlying, self.pass, *arg);
        let info = self.monosig.info(self.underlying, info);

        ir::EventBind {
            arg,
            info,
            delay,
            base,
        }
    }

    /// Either queues a connect to be handled later, or handles it now.
    /// A connect gets handled later if:
    /// (1) The source port is invoke-owned and the destination port is also invoke-owned, or
    /// (2) The source port is sig-owned and the destination port is invoke-owned
    /// A connect gets handled now if:
    /// (1) The source port is invoke-owned and the destination port is sig-owned
    /// (2) The source port is sig-owned and the destination port is also sig-owned
    /// The reasoning for this is that we see connects where the destination port is invoke-owned before
    /// we see the actual invoke, so we have to put them in a queue to be handled when we see the invoke
    /// If the destination port is not invoke owned, then we can handle it now because we've already created
    /// the monomorphized ports
    fn handle_connect(&mut self, con: &ir::Connect) -> Vec<ir::Command> {
        let src_port_owner = &self.underlying.get(con.src.port).owner;
        let dst_port_owner = &self.underlying.get(con.dst.port).owner;
        // We can elide the directions because at this point in the compiler they've already been verified to be correct,
        // basically we won't see anything wacky like sig.in <- inv.out
        match src_port_owner {
            ir::PortOwner::Inv { .. } => {
                match dst_port_owner {
                    ir::PortOwner::Inv { .. } => {
                        // handle later
                        self.monosig.connects.push(con.clone());
                        vec![]
                    }
                    ir::PortOwner::Sig { .. } => {
                        // handle now
                        let cmd = self.connect(con);
                        vec![cmd.into()]
                    }
                    ir::PortOwner::Local => panic!("aaaah"),
                }
            }
            ir::PortOwner::Sig { .. } => match dst_port_owner {
                ir::PortOwner::Inv { .. } => {
                    // handle later
                    self.monosig.connects.push(con.clone());
                    vec![]
                }
                ir::PortOwner::Sig { .. } => {
                    // handle now
                    let cmd = self.connect(con);
                    vec![cmd.into()]
                }
                ir::PortOwner::Local => panic!("aaaah"),
            },
            _ => panic!("ahhh"),
        }
    }

    fn command(&mut self, cmd: &ir::Command) -> Vec<ir::Command> {
        match cmd {
            ir::Command::Instance(idx) => vec![self.instance(*idx).into()],
            ir::Command::Invoke(idx) => vec![self.invoke(*idx).into()],
            ir::Command::Connect(con) => self.handle_connect(con),
            ir::Command::ForLoop(lp) => {
                self.forloop(lp);
                vec![]
            }
            ir::Command::If(if_stmt) => {
                self.if_stmt(if_stmt);
                vec![]
            }
            ir::Command::Fact(fact) => {
                if let Some(cmd) = self.fact(fact) {
                    vec![cmd]
                } else {
                    vec![]
                }
            }
        }
    }
}
