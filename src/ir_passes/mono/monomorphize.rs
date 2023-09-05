use super::{
    Base, CompKey, InstanceInfo, IntoBase, IntoUdl, MonoDeferred, MonoSig,
    UnderlyingComp,
};
use fil_ir::{self as ir, Ctx, IndexStore, MutCtx};
use std::collections::HashMap;

/// Information generated while monomorphizing a program. This tracks the global
/// set of information generated during monomorphization while [super::MonoSig]
/// tracks information generated for a single component.
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
    // pub queue: LinkedHashMap<CompKey, (Base<ir::Component>, MonoSig)>,

    /// Mapping from old ports to new ports, for resolving Foreigns
    inst_info: HashMap<CompKey, InstanceInfo>,

    /// Tracks which components are defined in which files
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
            inst_info: HashMap::new(),
            ext_map: HashMap::new(),
        }
    }
}

impl<'ctx> Monomorphize<'ctx> {
    pub fn inst_info(&self, comp_key: &CompKey) -> &InstanceInfo {
        self.inst_info.get(comp_key).unwrap()
    }

    pub fn inst_info_mut(&mut self, comp_key: CompKey) -> &mut InstanceInfo {
        self.inst_info.entry(comp_key).or_default()
    }

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

        // Otherwise, construct a new component and add it to the processing queue
        let new_comp = self.ctx.comp(underlying.is_ext).base();

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
        log::debug!("Signature of `{}'", comp.idx());
        MonoDeferred::sig(&mut monosig, underlying.clone(), self);

        let mut mono = MonoDeferred {
            underlying,
            pass: self,
            monosig,
        };

        log::debug!("Body of `{}'", key.comp.idx());
        mono.pass.processed.insert(key.clone(), new_comp);
        mono.gen_comp();
        let mut comp = mono.monosig.base;

        let default = self.ctx.get_mut(new_comp.get());
        comp.swap(default);
        let comp = comp.comp();
        ir::Validate::component(&self.ctx, comp);

        // return the `base` index so we can update the instance
        (new_comp, vec![])
    }

    // /// Monomorphize the next component in the stack.
    // fn next(&mut self) -> Option<(BaseComp, Base<ir::Component>)> {
    //     let Some((ck, (base_idx, monosig))) = self.queue.pop_front() else {
    //         return None;
    //     };

    //     let underlying = UnderlyingComp::new(self.old.get(ck.comp.idx()));
    //     let mut mono = MonoDeferred {
    //         underlying,
    //         pass: self,
    //         monosig,
    //     };

    //     log::debug!("Body of `{}'", ck.comp.idx());
    //     mono.pass.processed.insert(ck, base_idx);
    //     mono.gen_comp();
    //     let base = mono.monosig.base;

    //     // At this point, base_idx will be pointing to a default component
    //     // Return the idx so that we can swap them afterwards
    //     Some((base, base_idx))
    // }
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
        let entrypoint = entrypoint.ul();
        // Monomorphize the entrypoint
        let mut mono = Monomorphize::new(ctx);
        let ck = CompKey::new(entrypoint, vec![]);
        mono.should_process(ck.clone());

        let new_entrypoint = mono.processed.get(&ck).unwrap();
        mono.ctx.entrypoint = Some(new_entrypoint.get());
        mono.ctx.externals = mono.ext_map;
        mono.ctx
    }
}
