use super::{Attr, AttrCtx, AttrStore, BoolAttr, NumAttr};
use crate::GPosIdx;

/// Stores the attributes of a component
#[derive(Default, Clone)]
pub struct Attributes {
    /// Numerical attributes
    num_attrs: AttrStore<NumAttr, u64>,
    /// Boolean attributes
    bool_attrs: AttrStore<BoolAttr, bool>,
}

impl Attributes {
    pub fn new(attrs: impl Iterator<Item = (Attr, GPosIdx, u64)>) -> Self {
        let mut this = Self::default();
        for (attr, loc, value) in attrs {
            match attr {
                Attr::Num(attr) => this.set(attr, value, loc),
                Attr::Bool(attr) => this.set(attr, value != 0, loc),
            }
        }

        this
    }
}

impl AttrCtx<NumAttr, u64> for Attributes {
    fn get(&self, attr: NumAttr) -> Option<&u64> {
        self.num_attrs.get(attr)
    }

    fn get_loc(&self, attr: NumAttr) -> Option<GPosIdx> {
        self.num_attrs.get_loc(attr)
    }

    fn set(&mut self, attr: NumAttr, value: u64, loc: GPosIdx) {
        self.num_attrs.set(attr, value, loc);
    }

    fn remove(&mut self, attr: NumAttr) {
        self.num_attrs.remove(attr);
    }
}

impl AttrCtx<BoolAttr, bool> for Attributes {
    fn get(&self, attr: BoolAttr) -> Option<&bool> {
        self.bool_attrs.get(attr)
    }

    fn get_loc(&self, attr: BoolAttr) -> Option<GPosIdx> {
        self.bool_attrs.get_loc(attr)
    }

    fn set(&mut self, attr: BoolAttr, value: bool, loc: GPosIdx) {
        self.bool_attrs.set(attr, value, loc);
    }

    fn remove(&mut self, attr: BoolAttr) {
        self.bool_attrs.remove(attr);
    }
}
