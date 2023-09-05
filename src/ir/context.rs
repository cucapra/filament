use super::{AddCtx, CompIdx, Component, Ctx, Foreign, IndexStore, MutCtx};
use crate::utils::Idx;
use fil_derive::Ctx;
use std::collections::HashMap;

#[derive(Default, Ctx)]
pub struct Context {
    #[ctx(Component: Get, Add, Mut)]
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

    pub fn iter(&self) -> impl Iterator<Item = (CompIdx, &Component)> + '_ {
        self.comps.iter()
    }
}

impl<T> Ctx<T, Foreign<T, Component>> for Context
where
    Component: Ctx<T>,
{
    fn get(&self, idx: Foreign<T, Component>) -> &T {
        let (key, owner) = idx.take();

        self.get(owner).get(key)
    }
}
