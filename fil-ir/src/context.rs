use super::{
    AddCtx, CompIdx, Component, Ctx, Foreign, Idx, IndexStore, MutCtx,
};
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

    /// Is this component external?
    pub fn is_ext(&self, idx: CompIdx) -> bool {
        self.get(idx).is_ext()
    }

    /// Add a new component, default to the context
    pub fn comp(&mut self, is_ext: bool) -> CompIdx {
        let comp = Component::new(is_ext);
        self.add(comp)
    }

    /// Gets the filename the contains the external definition of a component.
    /// Returns `None` is the component is not an external
    pub fn get_filename(&self, idx: CompIdx) -> Option<String> {
        self.externals.iter().find_map(|(filename, comps)| {
            comps.contains(&idx).then_some(filename.to_string())
        })
    }

    /// Iterate over the components
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
