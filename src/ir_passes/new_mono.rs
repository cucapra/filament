use crate::ir::{self, Ctx, IndexStore, MutCtx};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

/// Represents a deferred monomorphize instance that will be processed by the pass at a later time.
struct MonoDeferred<'a, 'pass: 'a> {
    /// The name of the monomorphized component
    base: ir::Component,
    /// The underlying component to be monomorphized
    underlying: &'a ir::Component,
    /// Mapping from sig-owned parameters in the underlying component to their constant bindings.
    binding: ir::Bind<ir::ParamIdx, u64>,
    /// Mapping from non-sig-owned params in the underlying component to the parameters in the new component
    /// that they've been replaced with
    par_binding: ir::Bind<ir::ParamIdx, ir::ParamIdx>,

    /// Underlying pointer
    pass: &'a mut Monomorphize<'pass>,

    /// Mapping of underlying invokes (and how many times we've seen it) to base invokes
    inv_map: HashMap<(ir::InvIdx, u32), ir::InvIdx>,
    /// Mapping from underlying invokes to how many times we've seen it so far
    inv_counter: HashMap<ir::InvIdx, u32>,
    /// Hold onto the current PortIdx being handled
    curr_port: Option<ir::PortIdx>,
    /// Accesses
    connects: Vec<ir::Connect>,

    // Keep track of things that have been monomorphized already
    /// Events
    event_map: HashMap<ir::EventIdx, ir::EventIdx>,
    /// Times
    time_map: HashMap<ir::TimeIdx, ir::TimeIdx>,
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    fn gen_comp(&mut self) {
        for (idx, port) in self.underlying.ports().iter() {
            if port.is_sig_in() || port.is_sig_out() {
                self.port(idx);
            }
        }
        for cmd in &self.underlying.cmds {
            self.command(&cmd);
        }
    }

    /// Translates a ParamIdx defined by `underlying` to corresponding one in `base`
    /// Assumes that `param` is not sig-owned, because then it would be defined in the binding
    fn param(&mut self, param: ir::ParamIdx) -> ir::ParamIdx {
        let ir::Param { owner, .. } = self.underlying.get(param);

        match owner {
            ir::ParamOwner::Bundle(_) => {
                // this port idx is meaningful in self.underlying
                self.bundle_param(param)
            }
            ir::ParamOwner::Loop => {
                todo!()
            }
            ir::ParamOwner::Sig => {
                unreachable!("If a param is sig-owned, it should be resolved in the binding!")
            }
        }
    }

    /// Takes a self.underlying-owned param that is known to be bundle-owned and a port index owned by self.base,
    /// creates a new param that points to the port index, and adds the param to self.base. Returns the
    /// corresponding index
    fn bundle_param(&mut self, param: ir::ParamIdx) -> ir::ParamIdx {
        let ir::Param { info, .. } = self.underlying.get(param);

        let mono_owner = ir::ParamOwner::Bundle(self.curr_port.unwrap());
        let mono_param = ir::Param {
            owner: mono_owner,
            info: *info,
            default: None,
        };

        self.base.add(mono_param)
    }

    /// Translates an ExprIdx defined by `underlying` to correponding one in `base`.
    fn expr(&mut self, expr: ir::ExprIdx) -> ir::ExprIdx {
        match self.underlying.get(expr).clone() {
            ir::Expr::Param(p) => self
                .binding
                .get(&p)
                .map(|n| self.base.num(*n))
                .unwrap_or_else(|| self.param(p).expr(&mut self.base)),
            ir::Expr::Concrete(n) => self.base.num(n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(lhs);
                let rhs = self.expr(rhs);
                self.base.add(ir::Expr::Bin { op, lhs, rhs })
            }
            ir::Expr::Fn { op, args } => todo!(),
        }
    }

    /// Monomorphize the `inst` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn instance(&mut self, inst: ir::InstIdx) -> ir::InstIdx {
        let ir::Instance { comp, params, info } = self.underlying.get(inst);
        let conc_params = params
            .iter()
            .map(|p| self.expr(*p).as_concrete(&self.base).unwrap())
            .collect_vec();
        let (comp, params) = self.pass.should_process(*comp, conc_params);
        let new_inst = ir::Instance {
            comp,
            params: params.into_iter().map(|n| self.base.num(n)).collect(),
            info: *info,
        };
        self.base.add(new_inst)
    }

    /// Monomorphize the port (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn port(&mut self, port: ir::PortIdx) -> ir::PortIdx {
        let ir::Port {
            owner,
            width,
            live,
            info,
        } = self.underlying.get(port);

        let ir::Liveness {
            idx: underlying_idx,
            ..
        } = live;

        // Find the new port owner
        let mono_owner = self.find_new_portowner(owner);

        let info = info.clone();
        let info: ir::Info = self.underlying.get(info);
        let info_idx = self.base.add(info);

        // Add the new port so we can use its index in defining the correct Liveness
        let new_port = self.base.add(ir::Port {
            owner: mono_owner,
            width: *width,      // placeholder
            live: live.clone(), // placeholder
            info: info_idx
        });

        self.curr_port = Some(new_port);

        let ir::Liveness {
            idx,
            len,
            range
        } = live;
        

        let mono_liveness_idx = self.param(*idx);

        let mut mono_liveness = ir::Liveness {
            idx: mono_liveness_idx,
            len: len.clone(), // placeholder
            range: range.clone() // placeholder
        };

        //let mut mono_liveness = self.liveness(live, new_port);

        // Create a binding from old param -> new param
        self.par_binding.insert(*underlying_idx, mono_liveness_idx);

        // After making the new binding, re-monomorphize other parts of len in case they contained
        // the param we replaced
        let mono_width = self.expr(*width);
        mono_liveness.len = self.expr(mono_liveness.len);
        mono_liveness.range = self.range(&mono_liveness.range); // calling this is making us lookup nonexistent things

        let port = self.base.get_mut(new_port);
        port.live = mono_liveness; // update
        port.width = mono_width; // update

        new_port
    }

    /// Given a Liveness owned by underlying and a PortIdx meaningful in base, returns a Liveness that is meaningful in base
    fn liveness(
        &mut self,
        live: &ir::Liveness,
        port: ir::PortIdx,
    ) -> ir::Liveness {
        let ir::Liveness { idx, len, range } = live;
        let mono_idx = self.param(*idx);
        let mono_len = self.expr(*len);
        let mono_range = self.range(range);

        ir::Liveness {
            idx: mono_idx,
            len: mono_len,
            range: mono_range,
        }
    }

    /// Given a Range owned by underlying, returns a Range that is meaningful in base
    fn range(&mut self, range: &ir::Range) -> ir::Range {
        let ir::Range { start, end } = range;
        let start = self.time(*start);
        let end = self.time(*end);
        ir::Range {
            start,
            end
        }
    }

    /// Given an underlying PortOwner, returns the corresponding base PortOwner
    fn find_new_portowner(&mut self, owner: &ir::PortOwner) -> ir::PortOwner {
        match owner {
            ir::PortOwner::Sig { .. } | ir::PortOwner::Local => owner.clone(),
            ir::PortOwner::Inv { inv, dir } => {
                // inv is only meaningful in the underlying component
                let inv_occurrences = self.inv_counter.get(&inv).unwrap();
                let base_inv = match
                    self.inv_map.get(&(*inv, *inv_occurrences)) {
                        Some(n) => n,
                        None => {
                            println!("tried to get ({}, {}) in invmap", inv, inv_occurrences);
                            inv
                        }
                    };
                ir::PortOwner::Inv {
                    inv: *base_inv,
                    dir: dir.clone(),
                }
            }
        }
    }

    /// Monomorphize the event (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn event(&mut self, event: ir::EventIdx) -> ir::EventIdx {
        if let Some(idx) = self.event_map.get(&event) {
            return *idx
        };

        // Need to monomorphize all parts
        let ir::Event {
            delay,
            info,
            interface_port,
        } = self.underlying.get(event);

        let delay = self.delay(delay);
        let info = info.clone();
        let interface_port = match interface_port {
            Some(info) => Some(info.clone()),
            None => None
        };

        let idx = self.base.add(ir::Event {
            delay,
            info,interface_port
        });
        self.event_map.insert(event, idx);
        idx
    }

    /// Monomorphize the time (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn time(&mut self, time: ir::TimeIdx) -> ir::TimeIdx {
        if let Some(idx) = self.time_map.get(&time) {
            return *idx
        };

        let ir::Time { event, offset } = self.underlying.get(time);

        let mono_time = ir::Time {
            event: self.event(*event),
            offset: self.expr(*offset),
        };

        let idx = self.base.add(mono_time);
        self.time_map.insert(time, idx);
        idx
    }

    fn timesub(&mut self, timesub: &ir::TimeSub) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(expr) => ir::TimeSub::Unit(self.expr(*expr)),
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(*l),
                r: self.time(*r),
            },
        }
    }

    /// Monomorphize the delay (owned by self.underlying) and return one that is meaningful in `self.base`
    fn delay(&mut self, delay: &ir::TimeSub) -> ir::TimeSub {
        match delay {
            ir::TimeSub::Unit(expr) => ir::TimeSub::Unit(self.expr(*expr)),
            ir::TimeSub::Sym { l, r } => ir::TimeSub::Sym {
                l: self.time(*l),
                r: self.time(*r),
            },
        }
    }

    /// Monomorphize the `inv` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn invoke(&mut self, inv: ir::InvIdx) -> ir::InvIdx {
        // Count another time that we've seen inv
        self.insert_inv(inv);
        println!("inserted {} into counter", inv);
        

        // Handle connects we just saw:
        let connects = self.connects.clone();
        for con in connects.iter() {
            self.connect(con);
        }

        // Need to monomorphize all parts of the invoke
        let ir::Invoke {
            inst,
            ports,
            events,
            info,
        } = self.underlying.get(inv);

        // PLACEHOLDER, just want the index
        let mono_inv_idx = self.base.add(ir::Invoke {
            inst: *inst,
            ports: ports.clone(),
            events: events.clone(),
            info: *info
        });

        // Update the mapping from underlying invokes to base invokes
        // just unwrap because we maintain that inv will always be present in the mapping
        let inv_occurrences = self.inv_counter.get(&inv).unwrap();
        self.inv_map.insert((inv, *inv_occurrences), mono_inv_idx);

        // Instance - replace the instance owned by self.underlying with one owned by self.base

        // Ports
        let mono_ports = ports.iter().map(|p| { println!("{}", p); self.port(*p)}).collect_vec();

        // Events
        let mono_events =
            events.iter().map(|e| self.eventbind(e)).collect_vec();

        // Build the new invoke, add it to self.base
        let mut mono_inv = self.base.get_mut(mono_inv_idx);

        mono_inv.ports = mono_ports;
        mono_inv.events = mono_events;
        
        //println!("inserted ({}, {}) -> {} to invmap", inv, inv_occurrences, mono_inv_idx);

        mono_inv_idx
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

    fn prop(&mut self, prop: ir::PropIdx) -> ir::PropIdx {
        let prop = self.underlying.get(prop);
        self.base.add(prop.clone())
    }

    fn access(&mut self, acc: &ir::Access) -> ir::Access {
        let ir::Access { port, start, end } = acc;

        let port = self.port(*port);
        let start = self.expr(*start);
        let end = self.expr(*end);

        ir::Access {port,start,end}
    }

    fn connect(&mut self, con: &ir::Connect) -> ir::Connect {
        let ir::Connect { src, dst, info } = con;

        let mono_src = self.access(src);
        let mono_dst = self.access(dst);

        ir::Connect {
            src: mono_src,
            dst: mono_dst,
            info: info.clone(),
        }
    }

    fn forloop(&mut self, lp: &ir::Loop) -> ir::Loop {
        let ir::Loop {
            index,
            start,
            end,
            body,
        } = lp;

        let mono_index = self.param(*index);
        let mono_start = self.expr(*start);
        let mono_end = self.expr(*end);
        let mono_body = body.iter().map(|cmd| self.command(cmd)).filter(|c| c.is_some()).map(|c| c.unwrap()).collect_vec();

        ir::Loop {
            index: mono_index,
            start: mono_start,
            end: mono_end,
            body: mono_body,
        }
    }

    fn if_stmt(&mut self, if_stmt: &ir::If) -> ir::If {
        let ir::If { cond, then, alt } = if_stmt;

        let cond = self.prop(*cond);
        let then = then.iter().map(|cmd| self.command(cmd)).filter(|c| c.is_some()).map(|c| c.unwrap()).collect_vec();
        let alt = alt.iter().map(|cmd| self.command(cmd)).filter(|c| c.is_some()).map(|c| c.unwrap()).collect_vec();

        ir::If { cond, then, alt }
    }

    fn fact(&mut self, fact: &ir::Fact) -> ir::Fact {
        let ir::Fact { prop, reason, .. } = fact;

        let prop = self.prop(*prop);
        ir::Fact::assert(prop, reason.clone())
    }

    fn eventbind(&mut self, eb: &ir::EventBind) -> ir::EventBind {
        let ir::EventBind { arg, info, delay } = eb;

        let delay = self.timesub(delay);
        let arg = self.time(*arg);
        let info = info.clone();

        ir::EventBind { arg, info, delay }
    }

    fn command(&mut self, cmd: &ir::Command) -> Option<ir::Command> {
        match cmd {
            ir::Command::Instance(idx) => Some(self.instance(*idx).into()),
            ir::Command::Invoke(idx) => Some(self.invoke(*idx).into()),
            ir::Command::Connect(con) => { self.connects.push(con.clone()); None },
            ir::Command::ForLoop(lp) => Some(self.forloop(lp).into()),
            ir::Command::If(if_stmt) => Some(self.if_stmt(if_stmt).into()),
            ir::Command::Fact(fact) => Some(self.fact(fact).into()),
        }
    }

    fn take(self) -> ir::Component {
        self.base
    }
}

/// Monomorphize the Filament program
pub struct Monomorphize<'a> {
    /// The new context
    ctx: ir::Context,

    /// The old context
    old: &'a ir::Context,
    // Names of external components
    externals: Vec<ir::CompIdx>,

    /// Instances that have already been processed. Tracks the name of the generated component
    processed: HashMap<(ir::CompIdx, Vec<u64>), ir::CompIdx>,
    /// Instances that need to be generated
    queue: LinkedHashMap<(ir::CompIdx, Vec<u64>), ir::CompIdx>,
}

impl<'a> Monomorphize<'a> {
    fn new(old: &'a ir::Context) -> Self {
        Monomorphize {
            ctx: ir::Context {
                comps: IndexStore::default(),
                entrypoint: None,
            },
            old: &old,
            externals: vec![],
            processed: HashMap::new(),
            queue: LinkedHashMap::new(),
        }
    }
}

impl<'ctx> Monomorphize<'ctx> {
    /// Queue an instance for processing by the pass.
    /// The processing happens at a later point but, if needed, the pass immediately allocates a new [ir::Component] and returns information to construct a new instance.
    fn should_process(
        &mut self,
        comp: ir::CompIdx,
        params: Vec<u64>,
    ) -> (ir::CompIdx, Vec<u64>) {
        // If it is an external, add it to externals
        if self.old.get(comp).is_ext {
            self.externals.push(comp);
        }

        // If this component doesn't need monomorphization, return the comp index.
        if self.externals.contains(&comp) || !self.needs_monomorphize(comp) {
            println!("{} doesn't need mono", comp);
            return (comp, params);
        }
        let key = (comp, params);

        // If we've already processed this or queued this for processing, return the component
        if let Some(&name) =
            self.processed.get(&key).or_else(|| self.queue.get(&key))
        {
            return (name, vec![]);
        }

        // Otherwise, construct a new component and add it to the processing queue
        let new_comp = self.ctx.comp(false);
        self.queue.insert(key, new_comp);
        println!("put {} into queue; will generate {}", comp, new_comp);
        (new_comp, vec![])
    }

    fn next<'a>(&'a mut self) -> Option<ir::Component> {
        //Option<MonoDeferred<'ctx, 'a>> {
        let Some(((underlying, params), base)) = self.queue.pop_front() else {
            println!("returning None - nothing in queue");
            return None;
        };
        let underlying = self.old.get(underlying);
        let binding = underlying
            .sig_params()
            .into_iter()
            .zip(params)
            .collect_vec();
        let base = std::mem::take(self.ctx.get_mut(base));
        let mut mono = MonoDeferred {
            base,
            underlying,
            binding: ir::Bind::new(binding),
            par_binding: ir::Bind::new(vec![]),
            pass: self,
            inv_map: HashMap::new(),
            inv_counter: HashMap::new(),
            curr_port: None,
            connects: vec![],
            event_map: HashMap::new(),
            time_map: HashMap::new(),
        };
        mono.gen_comp();
        Some(mono.base)
    }

    /// Checks if a component needs to be monomorphized. This is the case if:
    /// - It has ANY parameters, or
    /// - If it uses loops, conditionals, or any other control constructs
    fn needs_monomorphize(&self, comp: ir::CompIdx) -> bool {
        let underlying = self.old.get(comp);

        let has_params = underlying
            .params()
            .iter()
            .fold(false, |acc, (_, param)| acc | param.is_sig_owned());

        let has_control = underlying
            .cmds
            .iter()
            .fold(false, |acc, cmd| acc | cmd.is_loop() | cmd.is_if());

        let has_insts = underlying
            .instances()
            .iter()
            .fold(false, |acc, (_, inst)| acc | (inst.params.len() != 0));

        has_params | has_control | has_insts
    }
}

impl Monomorphize<'_> {
    /// Monomorphize the context by tracing starting from the top-level component.
    /// Returns an empty context if there is no top-level component.
    pub fn transform(ctx: &ir::Context) -> ir::Context {
        let Some(entrypoint) = ctx.entrypoint else {
            log::warn!("program has no entrypoint! result will be empty");
            return ir::Context {
                comps: IndexStore::default(),
                entrypoint: None
            }
        };
        // Monomorphize the entrypoint
        let mut mono = Monomorphize::new(ctx);
        mono.should_process(entrypoint, vec![]);

        // Build a new context
        while let Some(comp) = mono.next() {
            let compidx = mono.ctx.add(comp);
            println!("added {} to ctx", compidx);
        }
        mono.ctx
    }
}
