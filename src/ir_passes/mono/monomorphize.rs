use super::{
    Base, CompKey, InstanceInfo, IntoBase, IntoUdl, MonoDeferred, MonoSig,
    UnderlyingComp,
};
use fil_ir::{self as ir, Ctx, IndexStore};
use ir::AddCtx;
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

    /// Monomorphize a component and return its index in the new context.
    pub fn monomorphize(&mut self, comp_key: CompKey) -> Base<ir::Component> {
        log::debug!("Monomorphizing `{}'", comp_key.comp.idx());
        let CompKey { comp, params } = comp_key;
        let underlying = self.old.get(comp.idx());

        let key: CompKey = if underlying.is_ext {
            (comp, vec![]).into()
        } else {
            (comp, params.clone()).into()
        };

        // If we've already processed this or queued this for processing, return the component
        if let Some(&name) = self.processed.get(&key) {
            return name;
        }

        // make a MonoSig
        let monosig = MonoSig::new(underlying, comp, underlying.is_ext, params);

        // the component whose signature we want to monomorphize
        // Monomorphize the sig
        let mono_comp = MonoDeferred {
            underlying: UnderlyingComp::new(self.old.get(comp.idx())),
            pass: self,
            monosig,
        }
        .comp();

        ir::Validate::component(&self.ctx, &mono_comp);
        let new_comp = self.ctx.add(mono_comp).base();
        self.processed.insert(key.clone(), new_comp);

        // `Some` if an extern, `None` if not
        if let Some(filename) = self.old.get_filename(comp.idx()) {
            if let Some(exts) = self.ext_map.get(&filename) {
                let mut exts = exts.clone();
                exts.push(new_comp.get());
                self.ext_map.insert(filename, exts.to_vec());
            } else {
                self.ext_map.insert(filename, vec![new_comp.get()]);
            }
        }

        // return the `base` index so we can update the instance
        new_comp
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
        let entrypoint = entrypoint.ul();
        // Monomorphize the entrypoint
        let mut mono = Monomorphize::new(ctx);
        let ck = CompKey::new(entrypoint, vec![]);
        mono.monomorphize(ck.clone());

        let new_entrypoint = mono.processed.get(&ck).unwrap();
        mono.ctx.entrypoint = Some(new_entrypoint.get());
        mono.ctx.externals = mono.ext_map;
        mono.ctx
    }
}
