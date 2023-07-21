use crate::ir::{self, Ctx, Foreign, MutCtx, DenseIndexInfo};
use itertools::Itertools;
use std::collections::HashMap;

use super::Monomorphize;

/// Used for monomorphizing a component's signature when we add it to the queue.
/// Any functions needed for monomorphizing the signature are located here - the rest are
/// in MonoDeferred.
pub struct MonoSig {
    /// The name of the monomorphized component
    pub base: ir::Component,
    /// The underlying component's idx
    pub underlying_idx: ir::CompIdx,
    /// Mapping from sig-owned parameters in the underlying component to their constant bindings.
    pub binding: ir::Bind<ir::ParamIdx, u64>,
    /// Mapping from non-sig-owned params in the underlying component to the parameters in the new component
    /// that they've been replaced with
    pub par_binding: ir::Bind<ir::ParamIdx, ir::ParamIdx>,

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
    /// Queue of connects to be handled when we see their associated invoke
    pub connects: Vec<ir::Connect>,

    // Keep track of things that have been monomorphized already
    /// Events
    pub event_map: DenseIndexInfo<ir::Event, ir::EventIdx>,
    /// Times
    pub time_map: DenseIndexInfo<ir::Time, ir::TimeIdx>,
    /// Ports
    pub port_map: DenseIndexInfo<(ir::Comp, ir::PortIdx), ir::PortIdx>,
    /// Exprs
    pub expr_map: DenseIndexInfo<ir::Expr, ir::ExprIdx>,
    /// Props
    pub prop_map: DenseIndexInfo<ir::Prop, ir::PropIdx>,
    /// Params
    pub param_map: DenseIndexInfo<ir::Param, ir::ParamIdx>,
}

impl MonoSig {
    pub fn new(
        base: ir::Component,
        underlying_idx: ir::CompIdx,
        binding: ir::Bind<ir::ParamIdx, u64>,
    ) -> Self {
        Self {
            base,
            underlying_idx,
            binding,
            par_binding: ir::Bind::new(vec![]),
            inv_map: HashMap::new(),
            inv_counter: HashMap::new(),
            inst_map: HashMap::new(),
            inst_counter: HashMap::new(),
            curr_port: None,
            connects: vec![],
            event_map: DenseIndexInfo::default(),
            time_map: DenseIndexInfo::default(),
            port_map: DenseIndexInfo::default(),
            expr_map: DenseIndexInfo::default(),
            prop_map: DenseIndexInfo::default(),
            param_map: DenseIndexInfo::default(),
        }
    }
}

impl MonoSig {
    /// Given an underlying PortOwner, returns the corresponding base PortOwner
    fn find_new_portowner(
        &mut self,
        underlying: &ir::Component,
        pass: &Monomorphize,
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
        pass: &Monomorphize,
        inv: &ir::InvIdx, // underlying
    ) -> Foreign<ir::Port, ir::Component> {
        // key is meaningful in underlying
        let Foreign { key, .. } = foreign;

        let inst = underlying.get(underlying.get(*inv).inst);
        let inst_comp = inst.comp;
        let inst_params = &inst.params;
        let conc_params = inst_params
            .iter()
            .map(|p| self.expr(underlying, *p).as_concrete(&self.base).unwrap())
            .collect_vec();

        let conc_params_copy = conc_params.clone();

        // now need to find the mapping from old portidx and the old instance to new port
        let new_port =
            pass.port_map.get(&(inst_comp, conc_params, *key)).unwrap();
        println!(
            "trying to get ({}, [{:?}], {}) from pass.port_map; got {}",
            inst_comp, conc_params_copy, key, new_port
        );

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
        info: &ir::InfoIdx,
    ) -> ir::InfoIdx {
        let info = underlying.get(*info);
        self.base.add(info.clone())
    }

    /// Translates a ParamIdx defined by `underlying` to corresponding one in `base`
    /// Assumes that `param` is not sig-owned, because then it would be defined in the binding
    fn param(
        &mut self,
        underlying: &ir::Component,
        param: ir::ParamIdx,
    ) -> ir::ParamIdx {
        if let Some(idx) = self.param_map.get(param) {
            return *idx;
        };
        let ir::Param { owner, info, .. } = underlying.get(param);

        match owner {
            ir::ParamOwner::Bundle(_) => {
                // this port idx is meaningful in self.underlying
                // let new_idx = self.bundle_param(param);
                // self.param_map.insert(param, new_idx);
                // new_idx
                if let Some(idx) = self.param_map.get(&param) {
                    return *idx;
                };
                let p = underlying.get(param);
                let new_idx = self.base.add(p.clone());
                self.param_map.insert(param, new_idx);
                new_idx
            }
            ir::ParamOwner::Loop => {
                let new_param = ir::Param {
                    owner: owner.clone(),
                    info: self.info(underlying, info),
                    default: None,
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
    pub fn expr(
        &mut self,
        underlying: &ir::Component,
        expr: ir::ExprIdx,
    ) -> ir::ExprIdx {
        if let Some(idx) = self.expr_map.get(&expr) {
            return *idx;
        };
        match underlying.get(expr).clone() {
            ir::Expr::Param(p) => {
                let new_idx = self
                    .binding
                    .get(&p)
                    .map(|n| self.base.num(*n))
                    .unwrap_or_else(|| {
                        self.param(underlying, p).expr(&mut self.base)
                    });
                self.expr_map.insert(expr, new_idx);
                new_idx
            }
            ir::Expr::Concrete(n) => {
                let new_idx = self.base.num(n);
                self.expr_map.insert(expr, new_idx);
                new_idx
            }
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(underlying, lhs);
                let rhs = self.expr(underlying, rhs);
                let new_expr = self.base.bin(ir::Expr::Bin { op, lhs, rhs });
                let new_idx = self.base.add(new_expr);
                self.expr_map.insert(expr, new_idx);
                new_idx
            }
            ir::Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|idx| self.expr(underlying, *idx))
                    .collect_vec();
                let new_expr = self.base.func(ir::Expr::Fn {
                    op,
                    args: Box::new(args),
                });
                let new_idx = self.base.add(new_expr);
                self.expr_map.insert(expr, new_idx);
                new_idx
            }
        }
    }

    /// Given a Range owned by underlying, returns a Range that is meaningful in base
    fn range(
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
            event: self.event(underlying, pass, *event),
            offset: self.expr(underlying, *offset),
        };

        let idx = self.base.add(mono_time);
        self.time_map.insert(time, idx);
        idx
    }

    /// Monomorphize the delay (owned by self.underlying) and return one that is meaningful in `self.base`
    fn delay(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        delay: &ir::TimeSub,
    ) -> ir::TimeSub {
        match delay {
            ir::TimeSub::Unit(expr) => {
                ir::TimeSub::Unit(self.expr(underlying, *expr))
            }
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(underlying, pass, *l),
                r: self.time(underlying, pass, *r),
            },
        }
    }

    /// Monomorphize the event (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn event(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        event: ir::EventIdx,
    ) -> ir::EventIdx {
        let binding = self.binding.inner();
        let conc_params = binding.iter().map(|(_, n)| *n).collect_vec();

        if let Some(idx) = self.event_map.get(&event) {
            pass.event_map
                .insert((self.underlying_idx, conc_params, event), *idx);
            return *idx;
        };

        // Need to monomorphize all parts
        let ir::Event {
            delay,
            info,
            has_interface,
        } = underlying.get(event);

        let delay = self.delay(underlying, pass, delay);
        let info = self.info(underlying, info);

        let idx = self.base.add(ir::Event {
            delay,
            info,
            has_interface: *has_interface,
        });
        // local event map
        self.event_map.insert(event, idx);

        // pass event map
        pass.event_map
            .insert((self.underlying_idx, conc_params, event), idx);
        idx
    }

    /// Takes a self.underlying-owned param that is known to be bundle-owned and a port index owned by self.base,
    /// creates a new param that points to the port index, and adds the param to self.base. Returns the
    /// corresponding index
    fn bundle_param(
        &mut self,
        underlying: &ir::Component,
        param: ir::ParamIdx,
    ) -> ir::ParamIdx {
        let ir::Param { info, .. } = underlying.get(param);
        let mono_info = self.info(underlying, info);
        let mono_owner = ir::ParamOwner::Bundle(self.curr_port.unwrap());

        if let Some(new_param_idx) = self.param_map.get(&param) {
            let mut new_param = self.base.get_mut(*new_param_idx);

            new_param.owner = mono_owner;
            new_param.info = mono_info;
            new_param.default = None;

            return *new_param_idx;
        };

        let mono_param = ir::Param {
            owner: mono_owner,
            info: self.info(underlying, info),
            default: None,
        };

        let new_idx = self.base.add(mono_param);
        self.param_map.insert(param, new_idx);
        new_idx
    }

    /// Monomorphize the port (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    pub fn port(
        &mut self,
        underlying: &ir::Component,
        pass: &mut Monomorphize,
        port: ir::PortIdx,
    ) -> ir::PortIdx {
        println!("{}", port);
        let ir::Port {
            owner,
            width,
            live,
            info,
        } = underlying.get(port);

        let comp_owner = match owner {
            ir::PortOwner::Sig { .. } => self.underlying_idx,
            ir::PortOwner::Inv { inv, .. } => {
                underlying.get(underlying.get(*inv).inst).comp
            }
            ir::PortOwner::Local => self.underlying_idx,
        };
        // port map for foreigns
        let binding = &self.binding.inner();
        let conc_params = binding.iter().map(|(_, n)| *n).collect_vec();

        if let Some(idx) = self.port_map.get(&(comp_owner, port)) {
            println!(
                "inserted ({}, [{:?}], {}) -> {} into pass.port_map",
                comp_owner, conc_params, port, *idx
            );
            pass.port_map.insert((comp_owner, conc_params, port), *idx);
            return *idx;
        };

        let ir::Liveness {
            idx: underlying_idx,
            ..
        } = live;

        let info = self.info(underlying, info);

        // Add the new port so we can use its index in defining the correct Liveness
        let new_port = self.base.add(ir::Port {
            owner: owner.clone(),
            width: *width,      // placeholder
            live: live.clone(), // placeholder
            info,
        });

        // local port map
        self.port_map.insert((comp_owner, port), new_port);

        // pass port map
        println!(
            "inserted ({}, [{:?}], {}) -> {} into pass.port_map",
            comp_owner, conc_params, port, new_port
        );
        pass.port_map
            .insert((comp_owner, conc_params, port), new_port);

        // Find the new port owner
        let mono_owner = self.find_new_portowner(underlying, pass, owner);

        let ir::Liveness { idx, len, range } = live;

        self.curr_port = Some(new_port);
        let mono_liveness_idx = self.bundle_param(underlying, *idx);

        let mut mono_liveness = ir::Liveness {
            idx: mono_liveness_idx,
            len: *len,            // placeholder
            range: range.clone(), // placeholder
        };

        // Create a binding from old param -> new param
        self.par_binding.insert(*underlying_idx, mono_liveness_idx);

        // After making the new binding, re-monomorphize other parts of len in case they contained
        // the param we replaced
        let mono_width = self.expr(underlying, *width);
        mono_liveness.len = self.expr(underlying, mono_liveness.len);
        mono_liveness.range =
            self.range(underlying, pass, &mono_liveness.range);

        let port = self.base.get_mut(new_port);
        port.live = mono_liveness; // update
        port.width = mono_width; // update
        port.owner = mono_owner; // update

        new_port
    }
}
