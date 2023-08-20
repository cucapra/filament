use super::{
    utils::{Base, Underlying},
    Monomorphize,
};
use crate::ir::{self, Ctx, DenseIndexInfo, Foreign, MutCtx};
use itertools::Itertools;
use std::collections::HashMap;

/// The port key is either a invocation port or a local port
type PortKey = (Option<Base<ir::Invoke>>, Underlying<ir::Port>);

type DenseMap<T> = DenseIndexInfo<T, Base<T>, Underlying<T>>;
type SparseMap<T> = HashMap<Underlying<T>, Base<T>>;

/// Used for monomorphizing a component's signature when we add it to the queue.
/// Any functions needed for monomorphizing the signature are located here - the rest are
/// in MonoDeferred.
pub struct MonoSig {
    /// The name of the monomorphized component
    pub base: ir::Component,
    /// The underlying component's idx
    pub underlying_idx: ir::CompIdx,
    /// Mapping from parameters in the underlying component to their constant bindings.
    pub binding: ir::Bind<Underlying<ir::Param>, u64>,

    /// Map from underlying invokes to base invokes
    pub invoke_map: DenseMap<ir::Invoke>,
    /// Map from underlying instances to base instances
    pub instance_map: DenseMap<ir::Instance>,

    // Keep track of things that have benen monomorphized already
    /// Events
    pub event_map: DenseMap<ir::Event>,
    /// Params - underlying param -> new Param. Kept in a sparse map because we're going to remove most parameters during monomorphization.
    pub param_map: SparseMap<ir::Param>,
    /// Bundle params - new port to new param
    pub bundle_param_map: HashMap<Base<ir::Port>, Base<ir::Param>>,
    /// Ports - (base inv, underlying port) -> base port
    pub port_map: HashMap<PortKey, Base<ir::Port>>,
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
                .map(Underlying::new)
                .zip(params)
                .collect_vec(),
        );
        Self {
            base,
            underlying_idx,
            binding,
            port_map: HashMap::new(),
            bundle_param_map: HashMap::new(),
            param_map: HashMap::default(),
            event_map: DenseMap::default(),
            invoke_map: DenseMap::default(),
            instance_map: DenseMap::default(),
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
                let base_inv = self.invoke_map.get(Underlying::new(*inv)).get();
                ir::PortOwner::Inv {
                    inv: base_inv,
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
        let key = Underlying::new(foreign.key());

        let inst = underlying.get(underlying.get(*inv).inst);
        let inst_comp = Underlying::new(inst.comp);

        let inst_params = &inst.params;
        let conc_params = inst_params
            .iter()
            .map(|p| {
                self.expr(underlying, Underlying::new(*p))
                    .get()
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();

        let conc_params = if pass.old.get(inst_comp.idx()).is_ext {
            vec![]
        } else {
            conc_params
        };

        let comp_k = (inst_comp, conc_params).into();

        let mono_compidx = if pass.queue.get(&comp_k).is_none() {
            pass.processed[&comp_k]
        } else {
            pass.queue[&comp_k].0
        };

        // now need to find the mapping from old portidx and the old instance to new port
        let global_port_map_k = (comp_k, key);
        let new_port = pass.port_map[&global_port_map_k];

        ir::Foreign::new(new_port.get(), mono_compidx.get())
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
            ir::info::Info::Assert(ir::info::Assert(reason)) => {
                ir::Info::assert(self.reason(underlying, pass, reason))
            }
            ir::info::Info::Param(_)
            | ir::Info::Instance(_)
            | ir::Info::Invoke(_)
            | ir::Info::Connect(_)
            | ir::Info::Port(_)
            | ir::Info::EventBind(_)
            | ir::Info::Event(_)
            | ir::Info::Empty(_) => info.clone(),
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
            } => ir::info::Reason::Liveness {
                dst_loc: *dst_loc,
                src_loc: *src_loc,
                dst_liveness: self.range(underlying, pass, dst_liveness),
                src_liveness: self.range(underlying, pass, src_liveness),
            },
            ir::info::Reason::ParamConstraint { .. }
            | ir::info::Reason::EventConstraint { .. }
            | ir::info::Reason::BundleLenMatch { .. }
            | ir::info::Reason::BundleWidthMatch { .. }
            | ir::info::Reason::InBoundsAccess { .. }
            | ir::info::Reason::BundleDelay { .. }
            | ir::info::Reason::WellFormedInterval { .. }
            | ir::info::Reason::EventTrig { .. }
            | ir::info::Reason::Misc { .. } => reason.clone(),
        }
    }

    /// Translates a ParamIdx defined by `underlying` to corresponding one in `base`
    /// Assumes that `param` is not sig-owned, because then it would be defined in the binding
    pub fn param_use(
        &mut self,
        underlying: &ir::Component,
        param: Underlying<ir::Param>,
    ) -> Base<ir::Param> {
        if let Some(&idx) = self.param_map.get(&param) {
            idx
        } else {
            // This param is a in a use site and should therefore have been found.
            let param = underlying.get(param.idx());
            match param.owner {
                ir::ParamOwner::Let | ir::ParamOwner::Loop => {
                    unreachable!("Let and loop params should should be resolved in the binding")
                }
                ir::ParamOwner::Bundle(_) => {
                    unreachable!("Bundle params should only be generated when visiting ports")
                }
                ir::ParamOwner::Sig => {
                    unreachable!("If a param is sig-owned, it should be resolved in the binding!")
                }
            }
        }
    }

    /// Translates an ExprIdx defined by `underlying` to correponding one in `base`.
    /// If the expression is a parameter bound in the bindings, then we return the bound value.
    /// Otherwise, we rewrite the expression and return the new index.
    ///
    /// We cannot cache the result of this function because the result depends
    /// on the binding in a particular scope.
    pub fn expr(
        &mut self,
        underlying: &ir::Component,
        expr: Underlying<ir::Expr>,
    ) -> Base<ir::Expr> {
        let e = underlying.get(expr.idx());

        // The expression is neither bound nor rewritten, so we need to rewrite it
        let new_idx = match e.clone() {
            ir::Expr::Param(p) => {
                // If this is a parameter in the underlying component that is bound,
                // return its binding
                if let Some(n) = self.binding.get(&Underlying::new(p)) {
                    let new_idx = Base::new(self.base.num(*n));
                    return new_idx;
                } else {
                    self.param_use(underlying, Underlying::new(p))
                        .get()
                        .expr(&mut self.base)
                }
            }
            ir::Expr::Concrete(n) => self.base.num(n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(underlying, Underlying::new(lhs)).get();
                let rhs = self.expr(underlying, Underlying::new(rhs)).get();
                let binop = ir::Expr::Bin { op, lhs, rhs };
                self.base.add(binop)
            }
            ir::Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|idx| {
                        self.expr(underlying, Underlying::new(*idx)).get()
                    })
                    .collect_vec();
                let func = ir::Expr::Fn { op, args };
                self.base.func(func)
            }
        };
        Base::new(new_idx)
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
            event: self.event(pass, Underlying::new(*event)).get(),
            offset: self.expr(underlying, Underlying::new(*offset)).get(),
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
            ir::TimeSub::Unit(expr) => ir::TimeSub::Unit(
                self.expr(underlying, Underlying::new(*expr)).get(),
            ),
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(underlying, pass, *l),
                r: self.time(underlying, pass, *r),
            },
        }
    }

    /// XXX(rachit): What does this method do?
    pub fn event_second(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        event: Underlying<ir::Event>,
        new_event: Base<ir::Event>,
    ) {
        let ir::Event { delay, info, .. } = underlying.get(event.idx());

        let delay = self.delay(underlying, pass, delay);
        let info = self.info(underlying, pass, info);

        let new_ev = self.base.get_mut(new_event.get());
        new_ev.delay = delay;
        new_ev.info = info;
    }

    pub fn interface(
        &mut self,
        underlying: &ir::Component,
        interface: &Option<ir::InterfaceSrc>,
    ) {
        self.base.src_info = interface.clone().map(
            |ir::InterfaceSrc {
                 name,
                 ports,
                 interface_ports,
                 params,
                 events,
             }| {
                let params = if underlying.is_ext {
                    params
                        .into_iter()
                        .map(|(p, id)| {
                            (self.param_map[&Underlying::new(p)].get(), id)
                        })
                        .collect()
                } else {
                    params
                };

                ir::InterfaceSrc {
                    name,
                    ports,
                    interface_ports: interface_ports
                        .into_iter()
                        .map(|(ev, id)| {
                            (self.event_map.get(Underlying::new(ev)).get(), id)
                        })
                        .collect(),
                    params,
                    events: events
                        .into_iter()
                        .map(|(ev, id)| {
                            (self.event_map.get(Underlying::new(ev)).get(), id)
                        })
                        .collect(),
                }
            },
        );
    }

    /// Monomorphize the event (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn event(
        &mut self,
        pass: &mut Monomorphize,
        event: Underlying<ir::Event>,
    ) -> Base<ir::Event> {
        let binding = self.binding.inner();
        let conc_params = binding.iter().map(|(_, n)| *n).collect_vec();

        let new_event = self.event_map.get(event);
        let ck = (Underlying::new(self.underlying_idx), conc_params).into();
        pass.event_map.insert((ck, event), *new_event);
        *new_event
    }

    /// Takes a underlying-owned param that is known to be bundle-owned and a port index owned by self.base,
    /// creates a new param that points to the port index, and adds the param to self.base. Returns the
    /// corresponding index
    fn bundle_param(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        param: Underlying<ir::Param>,
        port: Base<ir::Port>,
    ) -> Base<ir::Param> {
        let ir::Param { info, .. } = underlying.get(param.idx());
        let mono_info = self.info(underlying, pass, info);
        let mono_owner = ir::ParamOwner::Bundle(port.get());

        if let Some(new_param_idx) = self.bundle_param_map.get(&port) {
            let mut new_param = self.base.get_mut(new_param_idx.get());

            new_param.owner = mono_owner;
            new_param.info = mono_info;

            return *new_param_idx;
        };

        let mono_param = ir::Param {
            owner: mono_owner,
            info: self.info(underlying, pass, info),
        };

        let new_idx = Base::new(self.base.add(mono_param));
        self.param_map.insert(param, new_idx);
        self.bundle_param_map.insert(port, new_idx);
        new_idx
    }

    /// Monomorphize the definition of an invoke, add it to the base component,
    /// and return the corresponding base index
    pub fn inv_def(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inv: Underlying<ir::Invoke>,
    ) -> Base<ir::Invoke> {
        // Need to monomorphize all parts of the invoke
        let ir::Invoke {
            inst,
            ports,
            events,
            info,
        } = underlying.get(inv.idx());

        let info = self.info(underlying, pass, info);

        // PLACEHOLDER, just want the index when we add it to base
        let mono_inv_idx = Base::new(self.base.add(ir::Invoke {
            inst: *inst,
            ports: ports.clone(),
            events: events.clone(),
            info,
        }));

        // Update the mapping from underlying invokes to base invokes
        // just unwrap because we maintain that inv will always be present in the mapping
        self.invoke_map.insert(inv, mono_inv_idx);

        // Instance - replace the instance owned by self.underlying with one owned by self.base
        let base_inst = *self.instance_map.get(Underlying::new(*inst));

        // Ports
        let mono_ports = ports
            .iter()
            .map(|p| self.port_def(underlying, pass, Underlying::new(*p)).get())
            .collect_vec();

        // Events
        let mono_events = events
            .iter()
            .map(|e| self.eventbind(e, underlying, pass, inv))
            .collect_vec();

        // Build the new invoke, add it to self.base
        let mut mono_inv = self.base.get_mut(mono_inv_idx.get());

        mono_inv.inst = base_inst.get();
        mono_inv.ports = mono_ports;
        mono_inv.events = mono_events;

        mono_inv_idx
    }

    fn eventbind(
        &mut self,
        eb: &ir::EventBind,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inv: Underlying<ir::Invoke>,
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
            ir::TimeSub::Unit(expr) => ir::TimeSub::Unit(
                self.expr(underlying, Underlying::new(*expr)).get(),
            ),
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
        inv: Underlying<ir::Invoke>, // underlying
    ) -> Foreign<ir::Event, ir::Component> {
        let key = Underlying::new(foreign.key());
        // `key` is only meaningful in `owner`
        // need to map `key` to be the monomorphized index and update `owner` to be
        // the monomorphized component

        let inst = underlying.get(underlying.get(inv.idx()).inst);
        let inst_comp = Underlying::new(inst.comp);
        let inst_params = &inst.params;
        let conc_params = inst_params
            .iter()
            .map(|p| {
                self.expr(underlying, Underlying::new(*p))
                    .get()
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();

        let conc_params = if pass.old.get(inst_comp.idx()).is_ext {
            vec![]
        } else {
            conc_params
        };

        let ck = (inst_comp, conc_params).into();

        let new_owner = if pass.queue.get(&ck).is_none() {
            pass.processed[&ck]
        } else {
            pass.queue[&ck].0
        };

        let global_event_map_k = (ck, key);
        let new_event = pass.event_map.get(&global_event_map_k).unwrap();
        ir::Foreign::new(new_event.get(), new_owner.get())
    }

    /// Monomorphize the definition of an instance and add it to base component,
    /// and return the corresponding base index.
    pub fn inst_def(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        inst: Underlying<ir::Instance>,
    ) -> Base<ir::Instance> {
        let ir::Instance { comp, params, info } = underlying.get(inst.idx());
        let is_ext = pass.old.get(*comp).is_ext;
        let conc_params = params
            .iter()
            .map(|p| {
                self.expr(underlying, Underlying::new(*p))
                    .get()
                    .as_concrete(&self.base)
                    .unwrap()
            })
            .collect_vec();
        let ck = (Underlying::new(*comp), conc_params).into();
        let (comp, new_params) = pass.should_process(ck);

        let new_inst = if !is_ext {
            ir::Instance {
                comp: comp.get(),
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
                .map(|p| self.expr(underlying, Underlying::new(*p)).get())
                .collect_vec();
            ir::Instance {
                comp: comp.get(),
                params: ext_params.into(),
                info: self.info(underlying, pass, info),
            }
        };

        let new_idx = Base::new(self.base.add(new_inst));
        self.instance_map.insert(inst, new_idx);
        new_idx
    }

    /// For handling an external component's port. In this case, we don't want to replace parameters with concerete expressions.
    pub fn ext_port(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let ir::Port {
            owner,
            width,
            live,
            info,
        } = underlying.get(port.idx());

        let comp = Underlying::new(self.underlying_idx);

        let binding = &self.binding.inner();
        let cparams = if underlying.is_ext {
            vec![]
        } else {
            binding
                .iter()
                .filter(|(p, _)| underlying.get(p.idx()).is_sig_owned())
                .map(|(_, n)| *n)
                .collect_vec()
        };

        let port_map_k = (None, port);
        let global_port_map_k = ((comp, cparams).into(), port);

        // If the port has already been added to the base component, then we can
        // just return the index
        if let Some(idx) = self.port_map.get(&port_map_k) {
            pass.port_map.entry(global_port_map_k).or_insert(*idx);
            return *idx;
        };

        let info = self.info(underlying, pass, info);
        // Add the new port so we can use its index in defining the correct Liveness
        let new_port = Base::new(self.base.add(ir::Port {
            owner: owner.clone(),
            width: *width,      // placeholder
            live: live.clone(), // placeholder
            info,
        }));

        // local port map
        self.port_map.insert(port_map_k, new_port);

        // pass port map
        pass.port_map.entry(global_port_map_k).or_insert(new_port);

        // Find the new port owner
        let mono_owner = self.find_new_portowner(underlying, pass, owner);

        let ir::Liveness { idx, len, range } = live;

        let mono_liveness_idx = self
            .bundle_param(underlying, pass, Underlying::new(*idx), new_port)
            .get();

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
            event: self.event(pass, Underlying::new(*event)).get(),
            offset: self.base.add(underlying.get(*offset).clone()),
        };
        let start = self.base.add(start);

        let ir::Time { event, offset } = underlying.get(end);
        let end = ir::Time {
            event: self.event(pass, Underlying::new(*event)).get(),
            offset: self.base.add(underlying.get(*offset).clone()),
        };
        let end = self.base.add(end);

        mono_liveness.range = ir::Range { start, end };

        let port = self.base.get_mut(new_port.get());
        port.live = mono_liveness;
        port.width = mono_width;
        port.owner = mono_owner;

        new_port
    }

    /// Return base representation of a port that has already been monomorphized
    pub fn port_use(
        &mut self,
        underlying: &ir::Component,
        port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let inv = match &underlying.get(port.idx()).owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => None,
            ir::PortOwner::Inv { inv, .. } => {
                let base_inv = *self.invoke_map.get(Underlying::new(*inv));
                Some(base_inv)
            }
        };
        let port_map_k = (inv, Underlying::new(port.idx()));
        if let Some(idx) = self.port_map.get(&port_map_k) {
            *idx
        } else {
            unreachable!("port_use called for undefined port");
        }
    }

    /// Monomorphize the port (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn port_def(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let ir::Port {
            owner,
            width,
            live,
            info,
        } = underlying.get(port.idx());

        let inv = match owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => None,
            ir::PortOwner::Inv { inv, .. } => {
                let base_inv = *self.invoke_map.get(Underlying::new(*inv));
                Some(base_inv)
            }
        };

        let port_map_k = (inv, port);
        let info = self.info(underlying, pass, info);

        // Add the new port so we can use its index in defining the correct Liveness
        let new_port = Base::new(self.base.add(ir::Port {
            owner: owner.clone(),
            width: *width,      // placeholder
            live: live.clone(), // placeholder
            info,
        }));

        // Overwrite the value in the port map if any. This is okay because this
        // method can be called on local ports defined in iterative scopes.
        self.port_map.insert(port_map_k, new_port);

        // Find the new port owner
        let mono_owner = self.find_new_portowner(underlying, pass, owner);

        let ir::Liveness { idx, len, range } = live;

        let mono_liveness_idx = self.bundle_param(
            underlying,
            pass,
            Underlying::new(*idx),
            new_port,
        );

        let mut mono_liveness = ir::Liveness {
            idx: mono_liveness_idx.get(),
            len: *len,            // placeholder
            range: range.clone(), // placeholder
        };

        self.bundle_param_map.insert(new_port, mono_liveness_idx);
        let mono_width = self.expr(underlying, Underlying::new(*width));
        mono_liveness.len = self
            .expr(underlying, Underlying::new(mono_liveness.len))
            .get();
        mono_liveness.len =
            self.base.bin(self.base.get(mono_liveness.len).clone());
        mono_liveness.range =
            self.range(underlying, pass, &mono_liveness.range);

        let port = self.base.get_mut(new_port.get());
        port.live = mono_liveness; // update
        port.width = mono_width.get(); // update
        port.owner = mono_owner; // update

        new_port
    }
}
