use crate::{
    ir::{self, Ctx, Foreign, IndexStore, MutCtx},
    ir_passes::mono::monosig::MonoSig,
};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

use super::MonoDeferred;

/// Monomorphize the Filament program
pub struct Monomorphize<'a> {
    /// The new context
    pub ctx: ir::Context,

    /// The old context
    pub old: &'a ir::Context,
    // Names of external components
    pub externals: Vec<ir::CompIdx>,

    /// Instances that have already been processed. Tracks the name of the generated component
    pub processed: HashMap<(ir::CompIdx, Vec<u64>), ir::CompIdx>,
    /// Instances that need to be generated
    pub queue: LinkedHashMap<(ir::CompIdx, Vec<u64>), (ir::CompIdx, MonoSig)>,

    /// Mapping from old ports to new ports, for resolving Foreigns
    pub port_map: HashMap<(ir::CompIdx, Vec<u64>, ir::PortIdx), ir::PortIdx>,
    /// Mapping from old events to new events, for resolving Foreigns
    pub event_map: HashMap<(ir::CompIdx, Vec<u64>, ir::EventIdx), ir::EventIdx>,
}

impl<'a> Monomorphize<'a> {
    fn new(old: &'a ir::Context) -> Self {
        Monomorphize {
            ctx: ir::Context {
                comps: IndexStore::default(),
                entrypoint: None,
            },
            old,
            externals: vec![],
            processed: HashMap::new(),
            queue: LinkedHashMap::new(),
            port_map: HashMap::new(),
            event_map: HashMap::new(),
        }
    }
}

impl<'ctx> Monomorphize<'ctx> {
    /// Queue an instance for processing by the pass.
    /// The processing happens at a later point but, if needed, the pass immediately allocates a new [ir::Component] and returns information to construct a new instance.
    pub fn should_process(
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
            return (comp, params);
        }
        let key = (comp, params.clone());

        // If we've already processed this or queued this for processing, return the component
        if let Some(&name) = self.processed.get(&key) {
            return (name, vec![]);
        }

        if let Some((name, mono)) = self.queue.get(&key) {
            return (*name, vec![]);
        }

        // Otherwise, construct a new component and add it to the processing queue
        let new_comp = self.ctx.comp(false);

        // Stuff needed to construct a MonoSig
        let base = std::mem::take(self.ctx.get_mut(new_comp));
        let underlying = self.old.get(comp);
        let binding = underlying
            .sig_params()
            .into_iter()
            .zip(params)
            .collect_vec();

        // make a MonoSig
        let mut monosig = MonoSig::new(base, comp, ir::Bind::new(binding));

        let underlying = self.old.get(comp);

        // Monomorphize the sig
        MonoDeferred::sig(&mut monosig, underlying, self);

        // Insert into queue, with monosig so we can pick up where we left off when ready
        self.queue.insert(key, (new_comp, monosig));

        // return the `base` index so we can update the instance
        (new_comp, vec![])
    }

    fn next(&mut self) -> Option<(ir::Component, ir::CompIdx)> {
        //Option<MonoDeferred<'ctx, 'a>> {
        let Some(((underlying_idx, params), (base_idx, monosig))) = self.queue.pop_front() else {
            return None;
        };
        self.processed
            .insert((underlying_idx, params.clone()), base_idx);

        let underlying = self.old.get(underlying_idx);

        // after take(), idx will point to default component
        //let base = std::mem::take(self.ctx.get_mut(base_idx));

        let mut mono = MonoDeferred {
            underlying,
            pass: self,
            monosig,
        };
        println!("generating {}", base_idx);
        mono.gen_comp();
        let base = mono.monosig.base;

        // At this point, base_idx will be pointing to a default component
        // Return the idx so that we can swap them afterwards
        Some((base, base_idx))
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

        // for entrypoints that don't have parameters or control flow, but still need to be monomorphized
        // because they instantiate things that need to be monomorphized
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
        while let Some((mut comp, idx)) = mono.next() {
            let default = mono.ctx.get_mut(idx);
            std::mem::swap(&mut comp, default);
            let val = ir::Validate {
                comp: &comp,
                ctx: &mono.ctx.comps,
            };
            val.comp();
        }
        mono.ctx
    }
}

// static method on monodeferred?
