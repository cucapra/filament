use super::{
    Base, BaseComp, CompKey, IntoBase, IntoUdl, Monomorphize, Underlying,
    UnderlyingComp,
};
use fil_ir::{
    self as ir, AddCtx, Ctx, DenseIndexInfo, DisplayCtx, Foreign, MutCtx,
    SparseInfoMap,
};
use itertools::Itertools;
use std::collections::HashMap;

/// The port key is either a invocation port or a local port
type PortKey = (Option<Base<ir::Invoke>>, Underlying<ir::Port>);

type DenseMap<T> = DenseIndexInfo<T, Base<T>, Underlying<T>>;
type SparseMap<T> = SparseInfoMap<T, Base<T>, Underlying<T>>;

/// Tracks all the information generated while monomorphizing a component as well the component itself.
/// The maps are used to transform table data like parameters, events, and ports
/// from the underlying component to the generated component.
pub struct MonoSig {
    /// The name of the monomorphized component
    pub base: BaseComp,
    /// The underlying component's idx
    pub underlying_idx: Underlying<ir::Component>,
    /// Mapping from parameters in the underlying component to their constant bindings.
    pub binding: ir::Bind<Underlying<ir::Param>, u64>,

    // Keep track of things that have benen monomorphized already
    /// Events
    pub event_map: DenseMap<ir::Event>,
    /// Params - underlying param -> new Param. Kept in a sparse map because we're going to remove most parameters during monomorphization.
    pub param_map: SparseMap<ir::Param>,
    /// Ports - (base inv, underlying port) -> base port
    pub port_map: HashMap<PortKey, Base<ir::Port>>,
    /// Bundle params - new port to new param
    bundle_param_map: HashMap<Base<ir::Port>, Vec<Base<ir::Param>>>,

    /// Map from underlying invokes to base invokes
    invoke_map: DenseMap<ir::Invoke>,
    /// Map from underlying instances to base instances
    instance_map: DenseMap<ir::Instance>,
}

impl MonoSig {
    pub fn new(
        underlying: &ir::Component,
        typ: ir::CompType,
        idx: Underlying<ir::Component>,
        params: Vec<u64>,
    ) -> Self {
        let binding = ir::Bind::new(
            underlying
                .sig_params()
                .map(|p| p.ul())
                .zip(params)
                .collect_vec(),
        );
        let comp = ir::Component::new(typ, underlying.attrs.clone());

        Self {
            base: BaseComp::new(comp),
            underlying_idx: idx,
            binding,
            port_map: HashMap::new(),
            bundle_param_map: HashMap::new(),
            param_map: SparseInfoMap::default(),
            event_map: DenseMap::default(),
            invoke_map: DenseMap::default(),
            instance_map: DenseMap::default(),
        }
    }

    /// String representation for the binding for debug purposes
    pub fn binding_rep(&self, ul: &UnderlyingComp<'_>) -> String {
        self.binding
            .iter()
            .map(|(p, e)| format!("{}: {}", ul.display(*p), e))
            .join(", ")
    }

    /// Given an underlying PortOwner, returns the corresponding base PortOwner
    fn find_new_portowner(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        owner: &ir::PortOwner,
    ) -> ir::PortOwner {
        match owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => owner.clone(),
            ir::PortOwner::Inv { inv, dir, base } => {
                // inv is only meaningful in the underlying component
                let inv = inv.ul();
                let base = self.foreign_port(base, underlying, pass, inv);
                let base_inv = self.invoke_map.get(inv).get();
                ir::PortOwner::Inv {
                    inv: base_inv,
                    dir: dir.clone(),
                    base,
                }
            }
        }
    }

    /// Get the component associated with a foreign port in the new context. We
    /// need to look up the instance that this port is associated with because
    /// each set of parameters will generate a new component after mono runs.
    fn foreign_comp(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        inv: Underlying<ir::Invoke>,
    ) -> (CompKey, Base<ir::Component>) {
        let inst_ul = underlying.get(inv).inst.ul();
        let ir::Instance { comp, .. } = underlying.get(inst_ul);

        let comp_k = if pass.old.is_ext(*comp) {
            CompKey::new(comp.ul(), vec![])
        } else {
            self.comp_key(underlying, inst_ul, pass)
        };

        let Some(&comp) = pass.processed.get(&comp_k) else {
            unreachable!("component {comp_k} should have been monomorphized")
        };

        (comp_k, comp)
    }

    fn foreign_port(
        &mut self,
        foreign: &Foreign<ir::Port, ir::Component>, // underlying
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        inv: Underlying<ir::Invoke>,
    ) -> Foreign<ir::Port, ir::Component> {
        let (comp_k, mono_compidx) = self.foreign_comp(underlying, pass, inv);
        let key = foreign.key().ul();
        let inst_comp = comp_k.comp;
        // now need to find the mapping from old portidx and the old instance to new port
        let new_port =
            pass.inst_info(&comp_k).get_port(key).unwrap_or_else(|| {
                unreachable!(
                    "port {:?}.{} is missing from global map",
                    inst_comp.idx(),
                    pass.old.get(inst_comp.idx()).display(key.idx())
                )
            });

        ir::Foreign::new(new_port.get(), mono_compidx.get())
    }

    /// Add `self.underlying`'s info to `self.base`. Nothing else needs to be done because all the constructs
    /// we generate and add to `self.base` map to the same source-level info that they did in `self.underlying`
    pub fn info(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        iidx: Underlying<ir::Info>,
    ) -> Base<ir::Info> {
        let info = underlying.get(iidx);

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
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        reason: &ir::info::Reason,
    ) -> ir::info::Reason {
        match reason {
            ir::info::Reason::Liveness {
                src_loc,
                dst_liveness,
                src_liveness,
            } => ir::info::Reason::Liveness {
                src_loc: *src_loc,
                dst_liveness: self.range(underlying, pass, dst_liveness),
                src_liveness: self.range(underlying, pass, src_liveness),
            },
            ir::info::Reason::ParamConstraint { .. }
            | ir::info::Reason::ExistsConstraint { .. }
            | ir::info::Reason::EventConstraint { .. }
            | ir::info::Reason::BundleLenMatch { .. }
            | ir::info::Reason::BundleWidthMatch { .. }
            | ir::info::Reason::InBoundsAccess { .. }
            | ir::info::Reason::BundleDelay { .. }
            | ir::info::Reason::WellFormedInterval { .. }
            | ir::info::Reason::EventTrig { .. }
            | ir::info::Reason::EventLive { .. }
            | ir::info::Reason::EventLiveDelay { .. }
            | ir::info::Reason::Misc { .. }
            | ir::info::Reason::Generated { .. } => {
                ir::info::Reason::generated(
                    "Elaborated during monomorphization.".to_string(),
                    reason.clone(),
                )
            }
        }
    }

    /// Translates a ParamIdx defined by `underlying` to corresponding one in `base`
    /// Assumes that `param` is not sig-owned, because then it would be defined in the binding
    pub fn param_use(
        &mut self,
        ul: &UnderlyingComp,
        p_idx: Underlying<ir::Param>,
    ) -> Base<ir::Param> {
        if let Some(&idx) = self.param_map.find(p_idx) {
            idx
        } else {
            let p_rep = ul.display(p_idx);
            // This param is a in a use site and should therefore have been found.
            let msg = match ul.get(p_idx).owner {
                ir::ParamOwner::Loop => "loop-bound parameter".to_string(),
                ir::ParamOwner::Let { .. } => "let-bound parameter".to_string(),
                ir::ParamOwner::Bundle(_) => {
                    "bundle-bound parameter".to_string()
                }
                ir::ParamOwner::Sig => "signature-bound parameter".to_string(),
                ir::ParamOwner::Instance { inst, .. } => format!(
                    "parameter defined by instance `{}'",
                    ul.display(inst.ul())
                ),
                ir::ParamOwner::Exists { .. } => {
                    unreachable!(
                        "existential parameter `{}' occurred in a use location",
                        p_rep
                    )
                }
            };
            unreachable!(
                "{} `{}' should have been resolved in the binding but the binding was: [{}]",
                msg,
                p_rep,
                self.binding_rep(ul),
            )
        }
    }

    pub fn prop(
        &mut self,
        underlying: &UnderlyingComp,
        prop: Underlying<ir::Prop>,
        pass: &mut Monomorphize,
    ) -> Base<ir::Prop> {
        let prop = underlying.get(prop);
        match prop {
            ir::Prop::True | ir::Prop::False => self.base.add(prop.clone()),
            ir::Prop::Cmp(cmp) => {
                let ir::CmpOp { op, lhs, rhs } = cmp;
                let lhs = self.expr(underlying, lhs.ul(), pass).get();
                let rhs = self.expr(underlying, rhs.ul(), pass).get();
                self.base
                    .add(ir::Prop::Cmp(ir::CmpOp { op: *op, lhs, rhs }))
            }
            ir::Prop::TimeCmp(tcmp) => {
                let ir::CmpOp { op, lhs, rhs } = tcmp;
                let lhs = lhs.ul();
                let rhs = rhs.ul();
                let lhs = self.time(underlying, pass, lhs);
                let rhs = self.time(underlying, pass, rhs);
                self.base.add(ir::Prop::TimeCmp(ir::CmpOp {
                    op: *op,
                    lhs: lhs.get(),
                    rhs: rhs.get(),
                }))
            }
            ir::Prop::TimeSubCmp(tscmp) => {
                let ir::CmpOp { op, lhs, rhs } = tscmp;
                let lhs = self.timesub(underlying, pass, lhs);
                let rhs = self.timesub(underlying, pass, rhs);
                self.base.add(ir::Prop::TimeSubCmp(ir::CmpOp {
                    op: *op,
                    lhs,
                    rhs,
                }))
            }
            ir::Prop::Not(p) => {
                let p = p.ul();
                let new_p = self.prop(underlying, p, pass);
                self.base.add(ir::Prop::Not(new_p.get()))
            }
            ir::Prop::And(l, r) => {
                let l = l.ul();
                let r = r.ul();
                let l = self.prop(underlying, l, pass);
                let r = self.prop(underlying, r, pass);
                self.base.add(ir::Prop::And(l.get(), r.get()))
            }
            ir::Prop::Or(l, r) => {
                let l = l.ul();
                let r = r.ul();
                let l = self.prop(underlying, l, pass);
                let r = self.prop(underlying, r, pass);
                self.base.add(ir::Prop::Or(l.get(), r.get()))
            }
            ir::Prop::Implies(l, r) => {
                let l = l.ul();
                let r = r.ul();
                let l = self.prop(underlying, l, pass);
                // concretize the left expression first because if it is false the right might be invalid
                // For example, `(X > 0) & (X - 1 >= 0)` would fail due to overflow.
                match self.base.get(l) {
                    fil_ir::Prop::True => self.prop(underlying, r, pass),
                    fil_ir::Prop::False => self.base.add(ir::Prop::True),
                    _ => {
                        let r = self.prop(underlying, r, pass);
                        self.base.add(ir::Prop::Implies(l.get(), r.get()))
                    }
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
        underlying: &UnderlyingComp,
        expr: Underlying<ir::Expr>,
        pass: &mut Monomorphize,
    ) -> Base<ir::Expr> {
        let e = underlying.get(expr);

        // The expression is neither bound nor rewritten, so we need to rewrite it
        let new_idx: Base<ir::Expr> = match e.clone() {
            ir::Expr::Param(p) => {
                // If this is a parameter in the underlying component that is bound,
                // return its binding
                if let Some(n) = self.binding.get(&p.ul()) {
                    let new_idx = self.base.num(*n);
                    return new_idx;
                } else {
                    let p = self.param_use(underlying, p.ul()).get();
                    self.base.add(ir::Expr::Param(p))
                }
            }
            ir::Expr::Concrete(n) => self.base.num(n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(underlying, lhs.ul(), pass).get();
                let rhs = self.expr(underlying, rhs.ul(), pass).get();
                let binop = ir::Expr::Bin { op, lhs, rhs };
                self.base.add(binop)
            }
            ir::Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|idx| self.expr(underlying, idx.ul(), pass).get())
                    .collect_vec();
                let func = ir::Expr::Fn { op, args };
                self.base.func(func)
            }
            ir::Expr::If { cond, then, alt } => {
                let cond = self.prop(underlying, cond.ul(), pass).get();
                let then = self.expr(underlying, then.ul(), pass).get();
                let alt = self.expr(underlying, alt.ul(), pass).get();

                self.base.add(ir::Expr::If { cond, then, alt })
            }
        };
        new_idx
    }

    /// Given a Range owned by underlying, returns a Range that is meaningful in base
    pub fn range(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        range: &ir::Range,
    ) -> ir::Range {
        let ir::Range { start, end } = range;
        let start = start.ul();
        let end = end.ul();
        let start = self.time(underlying, pass, start);
        let end = self.time(underlying, pass, end);
        ir::Range {
            start: start.get(),
            end: end.get(),
        }
    }

    pub fn time(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        time: Underlying<ir::Time>,
    ) -> Base<ir::Time> {
        let ir::Time { event, offset } = underlying.get(time);

        let mono_time = ir::Time {
            event: self.event(pass, event.ul()).get(),
            offset: self.expr(underlying, offset.ul(), pass).get(),
        };

        self.base.add(mono_time)
    }

    /// Monomorphize the delay (owned by self.underlying) and return one that is meaningful in `self.base`
    pub fn delay(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        delay: &ir::TimeSub,
    ) -> ir::TimeSub {
        match delay {
            ir::TimeSub::Unit(expr) => {
                ir::TimeSub::Unit(self.expr(underlying, expr.ul(), pass).get())
            }
            ir::TimeSub::Sym { l, r } => {
                let l = l.ul();
                let r = r.ul();
                ir::TimeSub::Sym {
                    l: self.time(underlying, pass, l).get(),
                    r: self.time(underlying, pass, r).get(),
                }
            }
        }
    }

    /// Monomorphize the event delays.
    /// Event delays may mention other events (G: |L-G|) so first we need to
    /// monomorphize all the events and then monomorphize their delays.
    pub fn event_delay(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        event: Underlying<ir::Event>,
        new_event: Base<ir::Event>,
    ) {
        let ir::Event { delay, info, .. } = underlying.get(event);
        let info = info.ul();

        let delay = self.delay(underlying, pass, delay);
        let info = self.info(underlying, pass, info);

        let new_ev = self.base.get_mut(new_event);
        new_ev.delay = delay;
        new_ev.info = info.get();
    }

    pub fn interface(&mut self, interface: &Option<ir::InterfaceSrc>) {
        self.base.set_src_info(interface.clone().map(
            |ir::InterfaceSrc {
                 name,
                 ports,
                 interface_ports,
                 params,
                 events,
                 ..
             }| {
                ir::InterfaceSrc {
                    name,
                    ports,
                    interface_ports: interface_ports
                        .iter()
                        .map(|(ev, id)| {
                            (self.event_map.get(ev.ul()).get(), *id)
                        })
                        .collect(),
                    params,
                    events: events
                        .iter()
                        .map(|(ev, id)| {
                            (self.event_map.get(ev.ul()).get(), *id)
                        })
                        .collect(),
                    // Things do not need to be generated after monomorphize
                    gen_tool: None,
                }
            },
        ));
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
        let ck: CompKey = (self.underlying_idx, conc_params).into();
        pass.inst_info_mut(ck).add_event(event, *new_event);
        *new_event
    }

    /// Takes a underlying-owned params that are known to be bundle-owned and a port index owned by self.base,
    /// creates a new param that points to the port index, and adds the param to self.base. Returns the
    /// corresponding index
    fn bundle_params(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        params: &[Underlying<ir::Param>],
        port: Base<ir::Port>,
    ) -> Vec<Base<ir::Param>> {
        let mono_owner = ir::ParamOwner::Bundle(port.get());

        if self.bundle_param_map.contains_key(&port) {
            unreachable!("port {} already has bundle params", port.get());
        };

        let mono_params = params
            .iter()
            .map(|old_pidx| {
                let mono_param = ir::Param {
                    owner: mono_owner.clone(),
                    info: self
                        .info(
                            underlying,
                            pass,
                            underlying.get(*old_pidx).info.ul(),
                        )
                        .get(),
                };
                let new_idx = self.base.add(mono_param);
                self.param_map.push(*old_pidx, new_idx);
                new_idx
            })
            .collect_vec();

        self.bundle_param_map.insert(port, mono_params.clone());
        mono_params
    }

    /// Monomorphize the definition of an invoke, add it to the base component,
    /// and return the corresponding base index
    pub fn inv_def(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        inv: Underlying<ir::Invoke>,
    ) -> Base<ir::Invoke> {
        // Need to monomorphize all parts of the invoke
        let ir::Invoke {
            inst,
            ports,
            events,
            info,
        } = underlying.get(inv);

        let info = info.ul();
        let info = self.info(underlying, pass, info);

        // PLACEHOLDER, just want the index when we add it to base
        let mono_inv_idx = self.base.add(ir::Invoke {
            inst: *inst,
            ports: ports.clone(),
            events: events.clone(),
            info: info.get(),
        });

        // Update the mapping from underlying invokes to base invokes
        // just unwrap because we maintain that inv will always be present in the mapping
        self.invoke_map.insert(inv, mono_inv_idx);

        // Instance - replace the instance owned by self.underlying with one owned by self.base
        let base_inst = *self.instance_map.get(inst.ul());

        // Ports
        let mono_ports = ports
            .iter()
            .map(|p| self.local_port_def(underlying, pass, p.ul()).get())
            .collect_vec();

        // Events
        let mono_events = events
            .iter()
            .map(|e| self.eventbind(e, underlying, pass, inv))
            .collect_vec();

        // Build the new invoke, add it to self.base
        let mono_inv = self.base.get_mut(mono_inv_idx);

        mono_inv.inst = base_inst.get();
        mono_inv.ports = mono_ports;
        mono_inv.events = mono_events;

        mono_inv_idx
    }

    fn eventbind(
        &mut self,
        eb: &ir::EventBind,
        underlying: &UnderlyingComp,
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

        let arg = arg.ul();
        let arg = self.time(underlying, pass, arg);

        let info = info.ul();
        let info = self.info(underlying, pass, info);

        ir::EventBind {
            arg: arg.get(),
            info: info.get(),
            delay,
            base,
        }
    }

    pub fn timesub(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        timesub: &ir::TimeSub,
    ) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(expr) => {
                ir::TimeSub::Unit(self.expr(underlying, expr.ul(), pass).get())
            }
            ir::TimeSub::Sym { l, r } => {
                let l = l.ul();
                let r = r.ul();
                ir::TimeSub::Sym {
                    l: self.time(underlying, pass, l).get(),
                    r: self.time(underlying, pass, r).get(),
                }
            }
        }
    }

    /// Construct the [CompKey] for a instance in the underlying component
    fn comp_key(
        &mut self,
        underlying: &UnderlyingComp,
        iidx: Underlying<ir::Instance>,
        pass: &mut Monomorphize,
    ) -> CompKey {
        let ir::Instance { comp, args, .. } = underlying.get(iidx);

        let conc_params = args
            .iter()
            .map(|p| {
                self.expr(underlying, p.ul(), pass)
                    .get()
                    .as_concrete(self.base.comp())
                    .unwrap()
            })
            .collect_vec();
        CompKey::new(comp.ul(), conc_params)
    }

    fn foreign_event(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        foreign: &Foreign<ir::Event, ir::Component>,
        inv: Underlying<ir::Invoke>, // underlying
    ) -> Foreign<ir::Event, ir::Component> {
        let (ck, new_owner) = self.foreign_comp(underlying, pass, inv);

        let key = foreign.key().ul();
        let Some(new_event) = pass.inst_info(&ck).get_event(key) else {
            unreachable!("missing event `{}' for {ck}", foreign.key())
        };
        ir::Foreign::new(new_event.get(), new_owner.get())
    }

    /// Monomorphize the definition of an instance and add it to base component,
    /// and return the corresponding base index.
    pub fn inst_def(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        inst: Underlying<ir::Instance>,
    ) -> Base<ir::Instance> {
        let ir::Instance {
            comp,
            args,
            params,
            info,
            lives: live,
        } = underlying.get(inst);

        // Monomorphize the component
        let ck = self.comp_key(underlying, inst, pass);
        let mono_comp = pass.monomorphize(ck.clone());

        // Binding for parameters defined by this instance
        self.binding.extend(params.iter().map(|p| {
            let p = p.ul();
            let ir::ParamOwner::Instance { base, .. } = underlying.get(p).owner
            else {
                unreachable!("param should be owned by instance")
            };
            (
                p,
                pass.inst_info(&ck).get_exist_val(base.key().ul()).unwrap(),
            )
        }));

        // Parameters for the new component. We only preserve them for external calls.
        let conc_params: Box<[ir::ExprIdx]> = if pass.old.is_ext(*comp) {
            args.iter()
                .map(|p| self.expr(underlying, p.ul(), pass).get())
                .collect_vec()
                .into_boxed_slice()
        } else {
            Box::new([])
        };

        // this is an extern, so keep the params - need to get them into the new component though
        let lives = live
            .iter()
            .map(|l| self.range(underlying, pass, l))
            .collect_vec();
        let new_inst = ir::Instance {
            comp: mono_comp.get(),
            args: conc_params,
            info: self.info(underlying, pass, info.ul()).get(),
            params: Vec::new(),
            lives,
        };

        let new_idx = self.base.add(new_inst);
        self.instance_map.insert(inst, new_idx);

        new_idx
    }

    /// Return base representation of a port that has already been monomorphized
    pub fn port_use(
        &mut self,
        underlying: &UnderlyingComp,
        port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let inv = match &underlying.get(port).owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => None,
            ir::PortOwner::Inv { inv, .. } => {
                let base_inv = *self.invoke_map.get(inv.ul());
                Some(base_inv)
            }
        };
        let port_map_k = (inv, port.idx().ul());
        if let Some(idx) = self.port_map.get(&port_map_k) {
            *idx
        } else {
            unreachable!("port_use called for undefined port");
        }
    }

    /// Monomorphize a local port
    pub fn local_port_def(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let base = self.port_def_partial(underlying, pass, port);
        self.port_data(underlying, pass, port, base);
        base
    }

    /// Monomorphize the port (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn port_def_partial(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        port: Underlying<ir::Port>,
    ) -> Base<ir::Port> {
        let ir::Port {
            owner,
            width,
            live,
            info,
        } = underlying.get(port);
        let attrs = underlying.port_attrs().get(port.idx());

        let inv = match owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => None,
            ir::PortOwner::Inv { inv, .. } => {
                let base_inv = *self.invoke_map.get(inv.ul());
                Some(base_inv)
            }
        };

        let port_map_k = (inv, port);
        let info = info.ul();
        let info = self.info(underlying, pass, info);

        // Add the new port so we can use its index in defining the correct Liveness
        let new_port = self.base.add(ir::Port {
            owner: owner.clone(),
            width: *width,      // placeholder
            live: live.clone(), // placeholder
            info: info.get(),
        });

        self.base.push_port_attrs(new_port, attrs.clone());

        // Overwrite the value in the port map if any. This is okay because this
        // method can be called on local ports defined in iterative scopes.
        self.port_map.insert(port_map_k, new_port);

        new_port
    }

    /// Monomorphize [ir::Liveness] and width of a port.
    /// This is seperate from [port_def] because for signature ports, we need to
    /// do this after the body has been monomorphized. This is because port
    /// liveness and widths might mention existentially quantified parameters
    /// which only get their binding after a component has been monomorphized.
    pub fn port_data(
        &mut self,
        underlying: &UnderlyingComp,
        pass: &mut Monomorphize,
        port: Underlying<ir::Port>,
        new_port: Base<ir::Port>,
    ) {
        let ir::Port {
            owner, width, live, ..
        } = underlying.get(port);

        // Find the new port owner
        let mono_owner = self.find_new_portowner(underlying, pass, owner);

        let ir::Liveness { idxs, lens, range } = live;

        let mono_idxs = self.bundle_params(
            underlying,
            pass,
            &idxs.iter().map(|idx| idx.ul()).collect_vec(),
            new_port,
        );

        let mut mono_liveness = ir::Liveness {
            idxs: mono_idxs.iter().map(|idx| idx.get()).collect_vec(),
            lens: vec![],         // placeholder
            range: range.clone(), // placeholder
        };

        let mono_width = self.expr(underlying, width.ul(), pass);
        mono_liveness.lens = lens
            .iter()
            .map(|len| {
                let e = self.expr(underlying, len.ul(), pass).get();
                // Simplify the expression
                self.base.bin(self.base.get(e.base()).clone()).get()
            })
            .collect_vec();

        mono_liveness.range =
            self.range(underlying, pass, &mono_liveness.range);

        let port = self.base.get_mut(new_port);
        port.live = mono_liveness; // update
        port.width = mono_width.get(); // update
        port.owner = mono_owner; // update
    }
}
