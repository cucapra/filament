use super::{CompIdx, Component, Ctx, IndexStore, MutCtx};
use crate::utils::Idx;
use fil_derive::Ctx;
use std::collections::HashMap;

#[derive(Default, Ctx)]
pub struct Context {
    #[ctx(Component)]
    #[mut_ctx(Component)]
    pub comps: IndexStore<Component>,
    // Contains external components grouped by file name.
    pub externals: HashMap<String, Vec<CompIdx>>,
    pub entrypoint: Option<CompIdx>,
}

impl Context {
    pub fn is_main(&self, idx: CompIdx) -> bool {
        Some(idx) == self.entrypoint
    }

    /// Add a new component to the context
    pub fn comp(&mut self, is_ext: bool) -> CompIdx {
        let comp = Component::new(is_ext);
        self.add(comp)
    }

    pub fn get_filename(&self, idx: CompIdx) -> Option<String> {
        self.externals.iter().find_map(|(filename, comps)| {
            comps.contains(&idx).then_some(filename.to_string())
        })
    }
}
