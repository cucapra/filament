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

    pub invoke_map: HashMap<ir::InvIdx, ir::InvIdx>,
    pub instance_map: HashMap<ir::InstIdx, ir::InstIdx>,

    // Keep track of things that have benen moonmorphized already
    /// Events
    pub event_map: HashMap<ir::EventIdx, ir::EventIdx>,
    /// Ports - (base inv, underlying port) -> base port
    pub port_map: HashMap<(Option<ir::InvIdx>, ir::PortIdx), ir::PortIdx>,
    /// Params - underlying param -> new Param
    pub param_map: HashMap<ir::ParamIdx, ir::ParamIdx>,
    /// Bundle params - new port to new param
    pub bundle_param_map: HashMap<ir::PortIdx, ir::ParamIdx>,
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
            event_map: HashMap::new(),
            port_map: HashMap::new(),
            param_map: HashMap::new(),
            bundle_param_map: HashMap::new(),
            invoke_map: HashMap::new(),
            instance_map: HashMap::new(),
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
                let base_inv = self.invoke_map.get(inv).unwrap();
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
        let key = foreign.key();

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

        let conc_params = if pass.old.get(inst_comp).is_ext {
            vec![]
        } else {
            conc_params
        };

        // now need to find the mapping from old portidx and the old instance to new port
        let new_port = pass
            .port_map
            .get(&(inst_comp, conc_params.clone(), key))
            .unwrap();

        let mono_compidx =
            if pass.queue.get(&(inst_comp, conc_params.clone())).is_none() {
                pass.processed.get(&(inst_comp, conc_params)).unwrap()
            } else {
                &pass.queue.get(&(inst_comp, conc_params)).unwrap().0
            };

        ir::Foreign::new(*new_port, *mono_compidx)
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
            ir::info::Info::Param(param) => {
                let ir::info::Param { name, bind_loc } = param;
                ir::Info::param(*name, *bind_loc)
            }
            ir::info::Info::Assert(reason) => {
                let ir::info::Assert(reason) = reason;
                ir::Info::assert(self.reason(underlying, pass, reason))
            }
            ir::Info::Instance(instance) => {
                let ir::info::Instance {
                    name,
                    comp_loc,
                    bind_loc,
                } = instance;
                ir::Info::instance(*name, *comp_loc, *bind_loc)
            }
            ir::Info::Invoke(invoke) => {
                let ir::info::Invoke {
                    name,
                    inst_loc,
                    bind_loc,
                } = invoke;
                ir::Info::invoke(*name, *inst_loc, *bind_loc)
            }
            ir::Info::Connect(connect) => {
                let ir::info::Connect { src_loc, dst_loc } = connect;
                ir::Info::connect(*dst_loc, *src_loc)
            }
            ir::Info::Port(port) => {
                let ir::info::Port {
                    name,
                    bind_loc,
                    width_loc,
                    live_loc,
                } = port;
                ir::Info::port(*name, *bind_loc, *width_loc, *live_loc)
            }
            ir::Info::EventBind(eventbind) => {
                let ir::info::EventBind {
                    ev_delay_loc,
                    bind_loc,
                } = eventbind;
                ir::Info::event_bind(*ev_delay_loc, *bind_loc)
            }
            ir::Info::Event(event) => {
                let ir::info::Event {
                    name,
                    bind_loc,
                    delay_loc,
                    interface_name,
                    interface_bind_loc,
                } = event;
                ir::Info::event_explicit(
                    *name,
                    *bind_loc,
                    *delay_loc,
                    *interface_name,
                    *interface_bind_loc,
                )
            }
            ir::Info::Empty(_) => ir::Info::empty(),
        };

        self.base.add(info)
    }

    pub fn reason(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        reason: &ir::info::Reason,
    ) -> ir::info::Reason {
        match reason {
            ir::info::Reason::Liveness {
                dst_loc,
                src_loc,
                dst_liveness,
                src_liveness,
            } => {
                let dst_loc = *dst_loc;
                let src_loc = *src_loc;
                let dst_liveness = self.range(underlying, pass, dst_liveness);
                let src_liveness = self.range(underlying, pass, src_liveness);
                ir::info::Reason::Liveness {
                    dst_loc,
                    src_loc,
                    dst_liveness,
                    src_liveness,
                }
            }
            ir::info::Reason::ParamConstraint {
                bind_loc,
                constraint_loc,
            } => {
                let bind_loc = *bind_loc;
                let constraint_loc = *constraint_loc;
                ir::info::Reason::ParamConstraint {
                    bind_loc,
                    constraint_loc,
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
        let ir::Time { event, offset } = underlying.get(time);

        let mono_time = ir::Time {
            event: self.event(pass, *event),
            offset: self.expr(underlying, pass, *offset),
        };

        self.base.add(mono_time)
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

    pub fn interface(
        &mut self,
        underlying: &ir::Component,
        interface: &Option<ir::InterfaceSrc>,
    ) {
        let interface = match interface {
            None => None,
            Some(ir::InterfaceSrc {
                name,
                ports,
                interface_ports,
                params,
            }) => {
                let mut new_events = HashMap::new();
                for (event, id) in interface_ports.iter() {
                    let new_event = self.event_map.get(event).unwrap();
                    new_events.insert(*new_event, *id);
                }

                let mut new_params = HashMap::new();
                for (param, id) in params.iter() {
                    if underlying.is_ext {
                        let new_param = self.param_map.get(param).unwrap();
                        new_params.insert(*new_param, *id);
                    }
                }

                let params = if underlying.is_ext {
                    new_params
                } else {
                    params.clone()
                };

                Some(ir::InterfaceSrc {
                    name: *name,
                    ports: ports.clone(),
                    interface_ports: new_events,
                    params,
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
        inv: ir::InvIdx,
    ) -> ir::InvIdx {
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
        self.invoke_map.insert(inv, mono_inv_idx);

        // Instance - replace the instance owned by self.underlying with one owned by self.base
        let base_inst = *self.instance_map.get(inst).unwrap();

        // Ports
        let mono_ports = ports
            .iter()
            .map(|p| self.port(underlying, pass, *p))
            .collect_vec();

        // Events
        let mono_events = events
            .iter()
            .map(|e| self.eventbind(e, underlying, pass, inv))
            .collect_vec();

        // Build the new invoke, add it to self.base
        let mut mono_inv = self.base.get_mut(mono_inv_idx);

        mono_inv.inst = base_inst;
        mono_inv.ports = mono_ports;
        mono_inv.events = mono_events;

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
        timesub: &ir::TimeSub,
    ) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(expr) => {
                ir::TimeSub::Unit(self.expr(underlying, pass, *expr))
            }
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
        let key = foreign.key();
        // `key` is only meaningful in `owner`
        // need to map `key` to be the monomorphized index and update `owner` to be
        // the monomorphized component

        let inst = underlying.get(underlying.get(inv).inst);
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

        let conc_params = if pass.old.get(inst_comp).is_ext {
            vec![]
        } else {
            conc_params
        };

        let new_event = pass
            .event_map
            .get(&(inst_comp, conc_params.clone(), key))
            .unwrap();

        let new_owner =
            if pass.queue.get(&(inst_comp, conc_params.clone())).is_none() {
                pass.processed.get(&(inst_comp, conc_params)).unwrap()
            } else {
                &pass.queue.get(&(inst_comp, conc_params)).unwrap().0
            };

        ir::Foreign::new(*new_event, *new_owner)
    }

    /// Monomorphize the `inst` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn instance(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inst: ir::InstIdx,
    ) -> ir::InstIdx {
        let ir::Instance { comp, params, info } = underlying.get(inst);
        let is_ext = pass.old.get(*comp).is_ext;
        let conc_params = params
            .iter()
            .map(|p| {
                self.expr(underlying, pass, *p)
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();
        let (comp, new_params) = pass.should_process(*comp, conc_params);

        let new_inst = if !is_ext {
            ir::Instance {
                comp,
                params: new_params
                    .into_iter()
                    .map(|n| self.base.num(n))
                    .collect(),
                info: self.info(underlying, pass, info),
            }
        } else {
            // this is an extern, so keep the params - need to get them into the new component though
            let ext_params = params
                .iter()
                .map(|p| self.expr(underlying, pass, *p))
                .collect_vec();
            ir::Instance {
                comp,
                params: ext_params.into(),
                info: self.info(underlying, pass, info),
            }
        };

        let new_idx = self.base.add(new_inst);
        self.instance_map.insert(inst, new_idx);
        new_idx
    }

    /// For handling an external component's port. In this case, we don't want to replace parameters with concerete expressions.
    pub fn ext_port(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        port: ir::PortIdx,
    ) -> ir::PortIdx {
        let ir::Port {
            owner,
            width,
            live,
            info,
        } = underlying.get(port);

        let inv: Option<ir::InvIdx> = None;
        let comp = self.underlying_idx;

        let binding = &self.binding.inner();
        let cparams = if underlying.is_ext {
            vec![]
        } else {
            binding
                .iter()
                .filter(|(p, _)| underlying.get(*p).is_sig_owned())
                .map(|(_, n)| *n)
                .collect_vec()
        };

        if let Some(idx) = self.port_map.get(&(inv, port)) {
            if !pass.port_map.contains_key(&(comp, cparams.clone(), port)) {
                pass.port_map.insert((comp, cparams, port), *idx);
            }

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
        if !pass.port_map.contains_key(&(comp, cparams.clone(), port)) {
            pass.port_map.insert((comp, cparams, port), new_port);
        };

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

        // if there's parameters, we don't want to replace them for handling externs
        let mono_width = self.base.add(underlying.get(*width).clone());
        mono_liveness.len =
            self.base.add(underlying.get(mono_liveness.len).clone());

        let ir::Range { start, end } = mono_liveness.range;
        let ir::Time { event, offset } = underlying.get(start);
        let start = ir::Time {
            event: self.event(pass, *event),
            offset: self.base.add(underlying.get(*offset).clone()),
        };
        let start = self.base.add(start);

        let ir::Time { event, offset } = underlying.get(end);
        let end = ir::Time {
            event: self.event(pass, *event),
            offset: self.base.add(underlying.get(*offset).clone()),
        };
        let end = self.base.add(end);

        mono_liveness.range = ir::Range { start, end };

        let port = self.base.get_mut(new_port);
        port.live = mono_liveness;
        port.width = mono_width;
        port.owner = mono_owner;

        new_port
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
                let cparams = if underlying.is_ext {
                    vec![]
                } else {
                    binding
                        .iter()
                        .filter(|(p, _)| underlying.get(*p).is_sig_owned())
                        .map(|(_, n)| *n)
                        .collect_vec()
                };
                (None, self.underlying_idx, cparams)
            }
            ir::PortOwner::Inv { inv, .. } => {
                let inst = underlying.get(underlying.get(*inv).inst);
                let inst_comp = pass.old.get(inst.comp);

                let base_inv = *self.invoke_map.get(inv).unwrap();

                let conc_params = if inst_comp.is_ext {
                    vec![]
                } else {
                    inst.params
                        .iter()
                        .map(|p| {
                            self.expr(underlying, pass, *p)
                                .as_concrete(&self.base)
                                .unwrap()
                        })
                        .collect_vec()
                };
                (Some(base_inv), inst.comp, conc_params)
            }
        };

        if let Some(idx) = self.port_map.get(&(inv, port)) {
            pass.port_map
                .entry((comp, conc_params, port))
                .or_insert(port);
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
        pass.port_map
            .entry((comp, conc_params, port))
            .or_insert(new_port);

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
