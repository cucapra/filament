use crate::{attr::AttrCtx, GPosIdx};
use std::collections::HashMap;

/// A store for attributes
#[derive(Clone, PartialEq, Eq)]
pub struct AttrStore<Attr, Value>
where
    Attr: Eq + std::hash::Hash + Copy,
{
    attrs: HashMap<Attr, (Value, GPosIdx)>,
}

impl<Attr, Value> AttrStore<Attr, Value>
where
    Attr: Eq + std::hash::Hash + Copy,
{
    /// iterates over the attributes
    pub fn iter(&self) -> impl Iterator<Item = (Attr, &Value)> {
        self.attrs.iter().map(|(attr, (value, _))| (*attr, value))
    }
}

impl<Attr, Value> AttrCtx<Attr, Value> for AttrStore<Attr, Value>
where
    Attr: Eq + std::hash::Hash + Copy,
{
    fn get(&self, attr: Attr) -> Option<&Value> {
        self.attrs.get(&attr).map(|(value, _)| value)
    }

    fn get_loc(&self, attr: Attr) -> Option<GPosIdx> {
        self.attrs.get(&attr).map(|(_, loc)| *loc)
    }

    fn set(&mut self, attr: Attr, value: Value, loc: GPosIdx) {
        self.attrs.insert(attr, (value, loc));
    }

    fn remove(&mut self, attr: Attr) {
        self.attrs.remove(&attr);
    }
}

impl<Attr, Value> Default for AttrStore<Attr, Value>
where
    Attr: Eq + std::hash::Hash + Copy,
{
    fn default() -> Self {
        Self {
            attrs: HashMap::new(),
        }
    }
}
