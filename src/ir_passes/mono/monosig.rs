use super::Monomorphize;
use crate::ir::{self, Ctx, Foreign, MutCtx};
use itertools::Itertools;
use std::collections::HashMap;

/// Used for monomorphizing a component's signature when we add it to the queue.
/// Any functions needed for monomorphizing the signature are located here - the rest are
/// in MonoDeferred.
pub struct MonoSig {
    /// The name of the monomorphized component
    pub base: ir::Component,
    /// The underlying component's idx
    pub underlying_idx: ir::CompIdx,
    /// Mapping from parameters in the underlying component to their constant bindings.
    pub binding: ir::Bind<ir::ParamIdx, u64>,
    /// Queue of facts to monomorphize
    pub fact_queue: Vec<ir::Fact>,

    /// Mapping of underlying invokes (and how many times we've seen it) to base invokes
    pub inv_map: HashMap<(ir::InvIdx, u32), ir::InvIdx>,
    /// Mapping from underlying invokes to how many times we've seen it so far
    pub inv_counter: HashMap<ir::InvIdx, u32>,
    /// Mapping of underlying instances (and how many times we've seen it) to base instances
    pub inst_map: HashMap<(ir::InstIdx, u32), ir::InstIdx>,
    /// Maapping from underlying instances to how many times we've seen it so far
    pub inst_counter: HashMap<ir::InstIdx, u32>,
    /// Hold onto the current PortIdx being handled
    pub curr_port: Option<ir::PortIdx>,
    /// Invokes we've already handled, such as when we see an invoke's port before the actual invoke
    /// so we don't create duplicates
    pub handled_invokes: Vec<ir::InvIdx>,
    pub handled_instances: Vec<ir::InstIdx>,

    // Keep track of things that have benen moonmorphized already
    /// Events
    pub event_map: HashMap<ir::EventIdx, ir::EventIdx>,
    /// Times
    pub time_map: HashMap<ir::TimeIdx, ir::TimeIdx>,
    /// Ports - (base inv, underlying port) -> base port
    pub port_map: HashMap<(Option<ir::InvIdx>, ir::PortIdx), ir::PortIdx>,
    /// Exprs
    pub expr_map: HashMap<ir::ExprIdx, ir::ExprIdx>,
    /// Props
    pub prop_map: HashMap<ir::PropIdx, ir::PropIdx>,
    /// Params - underlying param -> new Param
    pub param_map: HashMap<ir::ParamIdx, ir::ParamIdx>,
    /// Bundle params - new port to new param
    pub bundle_param_map: HashMap<ir::PortIdx, ir::ParamIdx>,
    /// Info
    pub info_map: HashMap<ir::InfoIdx, ir::InfoIdx>,
}

impl MonoSig {
    pub fn new(
        base: &mut ir::Component,
        underlying: &ir::Component,
        underlying_idx: ir::CompIdx,
        params: Vec<u64>,
    ) -> Self {
        let base = std::mem::take(base);
        let binding = ir::Bind::new(
            underlying
                .sig_params()
                .into_iter()
                .zip(params)
                .collect_vec(),
        );
        Self {
            base,
            underlying_idx,
            binding,
            fact_queue: vec![],
            inv_map: HashMap::new(),
            inv_counter: HashMap::new(),
            inst_map: HashMap::new(),
            inst_counter: HashMap::new(),
            curr_port: None,
            event_map: HashMap::new(),
            time_map: HashMap::new(),
            port_map: HashMap::new(),
            expr_map: HashMap::new(),
            prop_map: HashMap::new(),
            param_map: HashMap::new(),
            bundle_param_map: HashMap::new(),
            info_map: HashMap::new(),
            handled_invokes: vec![],
            handled_instances: vec![],
        }
    }
}

impl MonoSig {
    /// Given an underlying PortOwner, returns the corresponding base PortOwner
    fn find_new_portowner(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        owner: &ir::PortOwner,
    ) -> ir::PortOwner {
        match owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => owner.clone(),
            ir::PortOwner::Inv { inv, dir, base } => {
                // inv is only meaningful in the underlying component
                let base = self.foreign_port(base, underlying, pass, inv);
                let inv_occurrences = self.inv_counter.get(inv).unwrap();
                let base_inv =
                    self.inv_map.get(&(*inv, *inv_occurrences)).unwrap();
                ir::PortOwner::Inv {
                    inv: *base_inv,
                    dir: dir.clone(),
                    base,
                }
            }
        }
    }

    fn foreign_port(
        &mut self,
        foreign: &Foreign<ir::Port, ir::Component>, // underlying
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inv: &ir::InvIdx, // underlying
    ) -> Foreign<ir::Port, ir::Component> {
        // key is meaningful in underlying
        let Foreign { key, .. } = foreign;

        let inst = underlying.get(underlying.get(*inv).inst);
        let inst_comp = inst.comp;
        let inst_params = &inst.params;
        let conc_params = inst_params
            .iter()
            .map(|p| {
                self.expr(underlying, pass, *p)
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();

        let conc_params_copy = conc_params.clone();

        // now need to find the mapping from old portidx and the old instance to new port
        let new_port =
            pass.port_map.get(&(inst_comp, conc_params, *key)).unwrap();

        // this will be the new Foreign's `owner`
        let (mono_compidx, _) =
            pass.queue.get(&(inst_comp, conc_params_copy)).unwrap();

        ir::Foreign {
            key: *new_port,
            owner: *mono_compidx,
        }
    }

    /// Add `self.underlying`'s info to `self.base`. Nothing else needs to be done because all the constructs
    /// we generate and add to `self.base` map to the same source-level info that they did in `self.underlying`
    pub fn info(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        iidx: &ir::InfoIdx,
    ) -> ir::InfoIdx {
        let info = underlying.get(*iidx);

        let info = match info {
            // ir::Info::Param {name, bind_loc} => {
            //     let param_size = self.param_counter;
            //     ir::Info::Param {name: ast::Id::new(format!("#_{param_size}")), bind_loc: bind_loc.clone()}
            // }
            ir::Info::Assert(reason) => {
                ir::Info::Assert(self.reason(underlying, pass, reason))
            }
            _ => info.clone(),
        };

        let new_idx = self.base.add(info);
        self.info_map.insert(*iidx, new_idx);
        new_idx
    }

    pub fn reason(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        reason: &ir::Reason,
    ) -> ir::Reason {
        match reason {
            ir::Reason::Liveness {
                dst_loc,
                src_loc,
                dst_liveness,
                src_liveness,
            } => {
                let dst_loc = *dst_loc;
                let src_loc = *src_loc;
                let dst_liveness = self.range(underlying, pass, dst_liveness);
                let src_liveness = self.range(underlying, pass, src_liveness);
                ir::Reason::Liveness {
                    dst_loc,
                    src_loc,
                    dst_liveness,
                    src_liveness,
                }
            }
            _ => reason.clone(),
        }
    }

    /// Translates a ParamIdx defined by `underlying` to corresponding one in `base`
    /// Assumes that `param` is not sig-owned, because then it would be defined in the binding
    pub fn param(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        param: ir::ParamIdx,
    ) -> ir::ParamIdx {
        if let Some(idx) = self.param_map.get(&param) {
            return *idx;
        };
        let ir::Param { owner, info, .. } = underlying.get(param);

        match owner {
            ir::ParamOwner::Bundle(_) => {
                unreachable!("Bundle params should only be generated when visiting ports")
            }
            ir::ParamOwner::Loop => {
                let new_param = ir::Param {
                    owner: owner.clone(),
                    info: self.info(underlying, pass, info),
                };
                let new_idx = self.base.add(new_param);
                self.param_map.insert(param, new_idx);
                new_idx
            }
            ir::ParamOwner::Sig => {
                unreachable!("If a param is sig-owned, it should be resolved in the binding!")
            }
        }
    }

    /// Translates an ExprIdx defined by `underlying` to correponding one in `base`.
    /// There are a couple of possibilities here:
    /// 1. This a parameter bound in the bindings
    /// 2. This has already been rewritten
    /// 3. This is the first time we're seen this expression
    pub fn expr(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        expr: ir::ExprIdx,
    ) -> ir::ExprIdx {
        // If this is a parameter in the underlying component that is bound,
        // return its binding
        let e = underlying.get(expr);
        if let ir::Expr::Param(p) = e {
            if let Some(n) = self.binding.get(p) {
                let new_idx = self.base.num(*n);
                return new_idx;
            }
        };
        // Check if we've already rewritten this expression
        // if let Some(idx) = self.expr_map.get(&expr) {
        //     return *idx;
        // };
        // The expression is neither bound nor rewritten, so we need to rewrite it
        let new_idx = match e.clone() {
            ir::Expr::Param(p) => {
                self.param(underlying, pass, p).expr(&mut self.base)
            }
            ir::Expr::Concrete(n) => self.base.num(n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(underlying, pass, lhs);
                let rhs = self.expr(underlying, pass, rhs);
                let binop = ir::Expr::Bin { op, lhs, rhs };
                self.base.add(binop)
            }
            ir::Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|idx| self.expr(underlying, pass, *idx))
                    .collect_vec();
                let func = ir::Expr::Fn { op, args };
                self.base.func(func)
            }
        };
        self.expr_map.insert(expr, new_idx);
        new_idx
    }

    /// Given a Range owned by underlying, returns a Range that is meaningful in base
    pub fn range(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        range: &ir::Range,
    ) -> ir::Range {
        let ir::Range { start, end } = range;
        let start = self.time(underlying, pass, *start);
        let end = self.time(underlying, pass, *end);
        ir::Range { start, end }
    }

    pub fn time(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        time: ir::TimeIdx,
    ) -> ir::TimeIdx {
        if let Some(idx) = self.time_map.get(&time) {
            return *idx;
        };

        let ir::Time { event, offset } = underlying.get(time);

        let mono_time = ir::Time {
            event: self.event(pass, *event),
            offset: self.expr(underlying, pass, *offset),
        };

        let idx = self.base.add(mono_time);
        self.time_map.insert(time, idx);

        idx
    }

    /// Monomorphize the delay (owned by self.underlying) and return one that is meaningful in `self.base`
    pub fn delay(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        delay: &ir::TimeSub,
    ) -> ir::TimeSub {
        match delay {
            ir::TimeSub::Unit(expr) => {
                ir::TimeSub::Unit(self.expr(underlying, pass, *expr))
            }
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(underlying, pass, *l),
                r: self.time(underlying, pass, *r),
            },
        }
    }

    pub fn event_second(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        event: ir::EventIdx,
        new_event: ir::EventIdx,
    ) -> ir::EventIdx {
        let ir::Event { delay, info, .. } = underlying.get(event);

        let delay = self.delay(underlying, pass, delay);
        let info = self.info(underlying, pass, info);

        let new_ev = self.base.get_mut(new_event);
        new_ev.delay = delay;
        new_ev.info = info;

        new_event
    }

    pub fn interface(&mut self, interface: &Option<ir::InterfaceSrc>) {
        let interface = match interface {
            None => None,
            Some(ir::InterfaceSrc {
                name,
                ports,
                interface_ports,
                params,
            }) => {
                // let mut new_ports = HashMap::new();
                // for (port, id) in ports.iter() {
                //     let new_port = self
                //         .port_map
                //         .get(&(self.underlying_idx, *port))
                //         .unwrap();
                //     new_ports.insert(*new_port, *id);
                // }

                let mut new_events = HashMap::new();
                for (event, id) in interface_ports.iter() {
                    let new_event = self.event_map.get(event).unwrap();
                    new_events.insert(*new_event, *id);
                }

                Some(ir::InterfaceSrc {
                    name: *name,
                    ports: ports.clone(),
                    interface_ports: new_events,
                    params: params.clone(),
                })
            }
        };
        self.base.src_info = interface;
    }

    /// Monomorphize the event (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn event(
        &mut self,
        pass: &mut Monomorphize,
        event: ir::EventIdx,
    ) -> ir::EventIdx {
        let binding = self.binding.inner();
        let conc_params = binding.iter().map(|(_, n)| *n).collect_vec();

        let new_event = self.event_map.get(&event).unwrap();
        pass.event_map
            .insert((self.underlying_idx, conc_params, event), *new_event);
        *new_event
    }

    /// Takes a self.underlying-owned param that is known to be bundle-owned and a port index owned by self.base,
    /// creates a new param that points to the port index, and adds the param to self.base. Returns the
    /// corresponding index
    fn bundle_param(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        param: ir::ParamIdx,
        port: ir::PortIdx, // base
    ) -> ir::ParamIdx {
        let ir::Param { info, .. } = underlying.get(param);
        let mono_info = self.info(underlying, pass, info);
        let mono_owner = ir::ParamOwner::Bundle(port);

        if let Some(new_param_idx) = self.bundle_param_map.get(&port) {
            let mut new_param = self.base.get_mut(*new_param_idx);

            new_param.owner = mono_owner;
            new_param.info = mono_info;

            return *new_param_idx;
        };

        let mono_param = ir::Param {
            owner: mono_owner,
            info: self.info(underlying, pass, info),
        };

        let new_idx = self.base.add(mono_param);
        self.param_map.insert(param, new_idx);
        self.bundle_param_map.insert(port, new_idx);
        new_idx
    }

    /// Monomorphize the `inv` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn invoke(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inv: ir::InvIdx
    ) -> ir::InvIdx {
        // Count another time that we've seen the underlying invoke
        self.insert_inv(inv);

        // Need to monomorphize all parts of the invoke
        let ir::Invoke {
            inst,
            ports,
            events,
            info,
        } = underlying.get(inv);

        let info = self.info(underlying, pass, info);

        // PLACEHOLDER, just want the index when we add it to base
        let mono_inv_idx = self.base.add(ir::Invoke {
            inst: *inst,
            ports: ports.clone(),
            events: events.clone(),
            info,
        });

        // Update the mapping from underlying invokes to base invokes
        // just unwrap because we maintain that inv will always be present in the mapping
        let inv_occurrences = self.inv_counter[&inv];
        self
            .inv_map
            .insert((inv, inv_occurrences), mono_inv_idx);

        // Instance - replace the instance owned by self.underlying with one owned by self.base
        let inst_occurrences = self.inst_counter[inst];
        let base_inst = self.inst_map[&(*inst, inst_occurrences)];

        // Ports
        let mono_ports = ports
            .iter()
            .map(|p| self.port(underlying, pass, *p))
            .collect_vec();

        // Events
        let mono_events =
            events.iter().map(|e| self.eventbind(e, underlying, pass, inv)).collect_vec();

        // Build the new invoke, add it to self.base
        let mut mono_inv = self.base.get_mut(mono_inv_idx);

        mono_inv.inst = base_inst;
        mono_inv.ports = mono_ports;
        mono_inv.events = mono_events;

        self.handled_invokes.push(inv);

        mono_inv_idx
    }

    fn eventbind(
        &mut self,
        eb: &ir::EventBind,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inv: ir::InvIdx,
    ) -> ir::EventBind {
        let ir::EventBind {
            arg,
            info,
            delay,
            base,
        } = eb;

        let base = self.foreign_event(underlying, pass, base, inv);
        let delay = self.timesub(underlying, pass, delay);
        let arg = self.time(underlying, pass, *arg);
        let info = self.info(underlying, pass, info);

        ir::EventBind {
            arg,
            info,
            delay,
            base,
        }
    }

    pub fn timesub(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        timesub: &ir::TimeSub
    ) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(expr) => ir::TimeSub::Unit(self.expr(
                underlying,
                pass,
                *expr,
            )),
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(underlying, pass, *l),
                r: self.time(underlying, pass, *r),
            },
        }
    }

    fn foreign_event(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        foreign: &Foreign<ir::Event, ir::Component>,
        inv: ir::InvIdx, // underlying
    ) -> Foreign<ir::Event, ir::Component> {
        let Foreign { key, .. } = foreign;
        // `key` is only meaningful in `owner`
        // need to map `key` to be the monomorphized index and update `owner` to be
        // the monomorphized component

        let inst = underlying.get(underlying.get(inv).inst);
        let inst_comp = inst.comp;
        let inst_params = &inst.params;
        let conc_params = inst_params
            .iter()
            .map(|p| {
                self
                    .expr(underlying, pass, *p)
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();

        let conc_params_copy = conc_params.clone();

        let new_event = pass
            .event_map
            .get(&(inst_comp, conc_params, *key))
            .unwrap();

        let new_owner = if let Some((mono_compidx, _)) =
            pass.queue.get(&(inst_comp, conc_params_copy))
        {
            *mono_compidx
        } else {
            self.underlying_idx
        };

        ir::Foreign {
            key: *new_event,
            owner: new_owner,
        }
    }

    /// Update the mapping of how many times we've seen each invoke in the underlying component.
    /// If the given invoke does not exist in the mapping, add it with a counter of 0
    /// If it does exist, increment the counter by 1
    fn insert_inv(&mut self, inv: ir::InvIdx) {
        if let Some(n) = self.inv_counter.get(&inv) {
            self.inv_counter.insert(inv, *n + 1);
        } else {
            self.inv_counter.insert(inv, 0);
        }
    }

    pub fn insert_inst(&mut self, inst: ir::InstIdx) {
        if let Some(n) = self.inst_counter.get(&inst) {
            self.inst_counter.insert(inst, *n + 1);
        } else {
            self.inst_counter.insert(inst, 0);
        }
    }

    /// Monomorphize the `inst` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn instance(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inst: ir::InstIdx
    ) -> ir::InstIdx {
        // Count another time we've seen the instance
        self.insert_inst(inst);

        let ir::Instance { comp, params, info } = underlying.get(inst);
        let conc_params = params
            .iter()
            .map(|p| {
                self
                    .expr(underlying, pass, *p)
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();
        let (comp, params) = pass.should_process(*comp, conc_params);
        let new_inst = ir::Instance {
            comp,
            params: params
                .into_iter()
                .map(|n| self.base.num(n))
                .collect(),
            info: self.info(underlying, pass, info),
        };

        let new_idx = self.base.add(new_inst);

        let inst_occurrences = self.inst_counter.get(&inst).unwrap();
        self
            .inst_map
            .insert((inst, *inst_occurrences), new_idx);
        self.handled_instances.push(inst);
        new_idx
    }

    /// Monomorphize the port (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn port(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        port: ir::PortIdx,
    ) -> ir::PortIdx {
        //self.insert_port(port);

        let ir::Port {
            owner,
            width,
            live,
            info,
        } = underlying.get(port);

        let (inv, comp, conc_params) = match owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => {
                let binding = &self.binding.inner();
                let cparams = binding
                    .iter()
                    .filter(|(p, _)| underlying.get(*p).is_sig_owned())
                    .map(|(_, n)| *n)
                    .collect_vec();
                (None, self.underlying_idx, cparams)
            }
            ir::PortOwner::Inv { inv, .. } => {
                let inst_idx = underlying.get(*inv).inst;
                let inst = underlying.get(underlying.get(*inv).inst);

                // its possible we haven't generated the new instance/invoke that this port belongs to
                // if inst_counter.get(inst).is_none() -> call instance
                if self.inst_counter.get(&inst_idx).is_none() {
                    self.instance(underlying, pass, inst_idx);
                };
                // if inv_counter.get(inv).is_none() -> call inv
                let base_inv = if self.inv_counter.get(inv).is_none() {
                    self.invoke(underlying, pass, *inv)
                } else {
                    let inv_occurrences = self.inv_counter.get(inv).unwrap();
                    *self.inv_map.get(&(*inv, *inv_occurrences)).unwrap()
                };

                let conc_params = inst
                    .params
                    .iter()
                    .map(|p| {
                        self.expr(underlying, pass, *p)
                            .as_concrete(&self.base)
                            .unwrap()
                    })
                    .collect_vec();
                // let inv_occurrences = self.inv_counter.get(inv).unwrap();
                // let base_inv =
                //     self.inv_map.get(&(*inv, *inv_occurrences)).unwrap();
                (Some(base_inv), inst.comp, conc_params)
            }
        };

        if let Some(idx) = self.port_map.get(&(inv, port)) {
            pass.port_map.insert((comp, conc_params, port), *idx);
            return *idx;
        };

        let info = self.info(underlying, pass, info);

        // Add the new port so we can use its index in defining the correct Liveness
        let new_port = self.base.add(ir::Port {
            owner: owner.clone(),
            width: *width,      // placeholder
            live: live.clone(), // placeholder
            info,
        });

        // local port map
        self.port_map.insert((inv, port), new_port);

        // pass port map
        pass.port_map.insert((comp, conc_params, port), new_port);

        // Find the new port owner
        let mono_owner = self.find_new_portowner(underlying, pass, owner);

        let ir::Liveness { idx, len, range } = live;

        let mono_liveness_idx =
            self.bundle_param(underlying, pass, *idx, new_port);

        let mut mono_liveness = ir::Liveness {
            idx: mono_liveness_idx,
            len: *len,            // placeholder
            range: range.clone(), // placeholder
        };

        self.bundle_param_map.insert(new_port, mono_liveness_idx);
        let mono_width = self.expr(underlying, pass, *width);
        mono_liveness.len = self.expr(underlying, pass, mono_liveness.len);
        mono_liveness.len =
            self.base.bin(self.base.get(mono_liveness.len).clone());
        mono_liveness.range =
            self.range(underlying, pass, &mono_liveness.range);

        let port = self.base.get_mut(new_port);
        port.live = mono_liveness; // update
        port.width = mono_width; // update
        port.owner = mono_owner; // update

        new_port
    }
}
