use crate::{
    ir::{self, Ctx, IndexStore, MutCtx},
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
                externals: HashMap::new(),
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

        let underlying = self.old.get(comp);
        let key = (comp, params.clone());

        // If we've already processed this or queued this for processing, return the component
        if let Some(&name) = self.processed.get(&key) {
            return (name, vec![]);
        }

        if let Some((name, _)) = self.queue.get(&key) {
            return (*name, vec![]);
        }


        // Otherwise, construct a new component and add it to the processing queue
        let new_comp = self.ctx.comp(underlying.is_ext);

        // // If this component doesn't need monomorphization, return the comp index. Still need to visit the signature
        // // though.
        // if self.externals.contains(&comp) || !self.needs_monomorphize(comp) {
        //     // for evidx in self.old.get(comp).events().idx_iter() {
        //     //     self.event_map.insert((new_comp, vec![], evidx), evidx);
        //     // }
        //     self.queue.insert(key, (new_comp, None));
        //     return (new_comp, params);
        // }

        let base = self.ctx.get_mut(new_comp);

        // make a MonoSig
        let mut monosig = MonoSig::new(base, underlying, comp, params.clone());

        // the component whose signature we want to monomorphize
        let underlying = self.old.get(comp);

        // Monomorphize the sig
        MonoDeferred::sig(&mut monosig, underlying, self);

        // Insert into queue, with monosig so we can pick up where we left off when ready
        self.queue.insert(key, (new_comp, monosig));

        // return the `base` index so we can update the instance
        (new_comp, vec![])
    }

    fn next(&mut self) -> Option<(ir::Component, ir::CompIdx)> {
        let Some(((underlying_idx, params), (base_idx, monosig))) = self.queue.pop_front() else {
            return None;
        };


        // let Some(monosig) = monosig else {
        //     // if no monosig was passed in, then we just need to copy everything in the original component over to the new one
        //     println!("copying {}", underlying_idx);
        //     let mut base = ir::Component::default();
        //     base.clone(underlying);
        //     return Some((base, base_idx));
        // };

        self.processed
            .insert((underlying_idx, params.clone()), base_idx);

        let underlying = self.old.get(underlying_idx);
        let mut mono = MonoDeferred {underlying, pass: self, monosig };

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
                entrypoint: None,
                externals: HashMap::new()
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
