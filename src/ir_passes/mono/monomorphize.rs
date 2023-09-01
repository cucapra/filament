use crate::{
    ir::{self, Ctx, IndexStore, MutCtx},
    ir_passes::mono::monosig::MonoSig,
};
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

use super::{
    monodeferred::MonoDeferred,
    utils::{Base, BaseComp, Underlying, UnderlyingComp},
};

#[derive(PartialEq, Eq, Hash, Clone)]
/// A key defined by a component and all of its parameters.
pub struct CompKey {
    pub comp: Underlying<ir::Component>,
    pub params: Vec<u64>,
}
impl CompKey {
    pub fn new(comp: Underlying<ir::Component>, params: Vec<u64>) -> Self {
        Self { comp, params }
    }
}
impl From<(Underlying<ir::Component>, Vec<u64>)> for CompKey {
    fn from((comp, params): (Underlying<ir::Component>, Vec<u64>)) -> Self {
        Self::new(comp, params)
    }
}

type PortKey = (CompKey, Underlying<ir::Port>);
type EventKey = (CompKey, Underlying<ir::Event>);

/// Monomorphize the Filament program
pub struct Monomorphize<'a> {
    /// The new context
    pub ctx: ir::Context,

    /// The old context
    pub old: &'a ir::Context,
    // Names of external components
    pub externals: Vec<ir::CompIdx>,

    /// Instances that have already been processed. Tracks the name of the generated component
    pub processed: HashMap<CompKey, Base<ir::Component>>,
    /// Instances that need to be generated
    pub queue: LinkedHashMap<CompKey, (Base<ir::Component>, MonoSig)>,

    /// Mapping from old ports to new ports, for resolving Foreigns
    pub port_map: HashMap<PortKey, Base<ir::Port>>,
    /// Mapping from old events to new events, for resolving Foreigns
    pub event_map: HashMap<EventKey, Base<ir::Event>>,

    pub ext_map: HashMap<String, Vec<ir::CompIdx>>,
}

impl<'a> Monomorphize<'a> {
    fn new(old: &'a ir::Context) -> Self {
        Monomorphize {
            ctx: ir::Context {
                comps: IndexStore::default(),
                entrypoint: None,
                externals: HashMap::new(),
            },
            old,
            externals: vec![],
            processed: HashMap::new(),
            queue: LinkedHashMap::new(),
            port_map: HashMap::new(),
            event_map: HashMap::new(),
            ext_map: HashMap::new(),
        }
    }
}

impl<'ctx> Monomorphize<'ctx> {
    /// Queue an instance for processing by the pass.
    /// The processing happens at a later point but, if needed, the pass immediately allocates a new [ir::Component] and returns information to construct a new instance.
    pub fn should_process(
        &mut self,
        comp_key: CompKey,
    ) -> (Base<ir::Component>, Vec<u64>) {
        let CompKey { comp, params } = comp_key;
        let underlying = self.old.get(comp.idx());

        let key: CompKey = if underlying.is_ext {
            (comp, vec![]).into()
        } else {
            (comp, params.clone()).into()
        };

        // If we've already processed this or queued this for processing, return the component
        if let Some(&name) = self.processed.get(&key) {
            return (name, vec![]);
        }

        if let Some((name, _)) = self.queue.get(&key) {
            return (*name, vec![]);
        }

        // Otherwise, construct a new component and add it to the processing queue
        let new_comp = Base::new(self.ctx.comp(underlying.is_ext));

        // `Some` if an extern, `None` if not
        let filename = self.old.get_filename(comp.idx());
        if let Some(filename) = filename {
            if let Some(exts) = self.ext_map.get(&filename) {
                let mut exts = exts.clone();
                exts.push(new_comp.get());
                self.ext_map.insert(filename, exts.to_vec());
            } else {
                self.ext_map.insert(filename, vec![new_comp.get()]);
            }
        }

        let base = self.ctx.get_mut(new_comp.get());

        // make a MonoSig
        let mut monosig = MonoSig::new(base, underlying, comp.idx(), params);

        // the component whose signature we want to monomorphize
        let underlying = UnderlyingComp::new(self.old.get(comp.idx()));

        // Monomorphize the sig
        MonoDeferred::sig(&mut monosig, underlying, self);

        // Insert into queue, with monosig so we can pick up where we left off when ready
        self.queue.insert(key, (new_comp, monosig));

        // return the `base` index so we can update the instance
        (new_comp, vec![])
    }

    fn next(&mut self) -> Option<(BaseComp, Base<ir::Component>)> {
        let Some((ck, (base_idx, monosig))) = self.queue.pop_front() else {
            return None;
        };

        let underlying = UnderlyingComp::new(self.old.get(ck.comp.idx()));
        let mut mono = MonoDeferred {
            underlying,
            pass: self,
            monosig,
        };

        mono.pass.processed.insert(ck, base_idx);
        mono.gen_comp();
        let base = mono.monosig.base;

        // At this point, base_idx will be pointing to a default component
        // Return the idx so that we can swap them afterwards
        Some((base, base_idx))
    }
}

impl Monomorphize<'_> {
    /// Monomorphize the context by tracing starting from the top-level component.
    /// Returns an empty context if there is no top-level component.
    pub fn transform(ctx: &ir::Context) -> ir::Context {
        let Some(entrypoint) = ctx.entrypoint else {
            log::warn!("Program has no entrypoint. Result will be empty.");
            return ir::Context {
                comps: IndexStore::default(),
                entrypoint: None,
                externals: HashMap::new(),
            };
        };
        let entrypoint = Underlying::new(entrypoint);
        // Monomorphize the entrypoint
        let mut mono = Monomorphize::new(ctx);
        let ck = CompKey::new(entrypoint, vec![]);
        mono.should_process(ck.clone());

        // Build a new context
        while let Some((mut comp, idx)) = mono.next() {
            let default = mono.ctx.get_mut(idx.get());
            comp.swap(default);
            let comp = comp.comp();
            let val = ir::Validate::new(comp, &mono.ctx.comps);
            val.comp();
        }
        let new_entrypoint = mono.processed.get(&ck).unwrap();
        mono.ctx.entrypoint = Some(new_entrypoint.get());
        mono.ctx.externals = mono.ext_map;
        mono.ctx
    }
}
