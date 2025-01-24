use super::{AttrCtx, AttrStore};
use crate::GPosIdx;
use std::{hash::Hash, str::FromStr};

/// Stores the attributes of a component
#[derive(Clone)]
pub struct Attributes<Bool, Num, Float>
where
    Bool: FromStr + Hash + Eq + Copy,
    Num: FromStr + Hash + Eq + Copy,
    Float: FromStr + Hash + Eq + Copy,
{
    /// Numerical attributes
    num_attrs: AttrStore<Num, u64>,
    /// Boolean attributes
    bool_attrs: AttrStore<Bool, bool>,
    /// Float attributes
    float_attrs: AttrStore<Float, f64>,
}

impl<Bool, Num, Float> AttrCtx<Num, u64> for Attributes<Bool, Num, Float>
where
    Bool: FromStr + Hash + Eq + Copy,
    Num: FromStr + Hash + Eq + Copy,
    Float: FromStr + Hash + Eq + Copy,
{
    fn get(&self, attr: Num) -> Option<&u64> {
        self.num_attrs.get(attr)
    }

    fn get_loc(&self, attr: Num) -> Option<GPosIdx> {
        self.num_attrs.get_loc(attr)
    }

    fn set(&mut self, attr: Num, value: u64, loc: GPosIdx) {
        self.num_attrs.set(attr, value, loc);
    }

    fn remove(&mut self, attr: Num) {
        self.num_attrs.remove(attr);
    }
}

impl<Bool, Num, Float> AttrCtx<Bool, bool> for Attributes<Bool, Num, Float>
where
    Bool: FromStr + Hash + Eq + Copy,
    Num: FromStr + Hash + Eq + Copy,
    Float: FromStr + Hash + Eq + Copy,
{
    fn get(&self, attr: Bool) -> Option<&bool> {
        self.bool_attrs.get(attr)
    }

    fn get_loc(&self, attr: Bool) -> Option<GPosIdx> {
        self.bool_attrs.get_loc(attr)
    }

    fn set(&mut self, attr: Bool, value: bool, loc: GPosIdx) {
        self.bool_attrs.set(attr, value, loc);
    }

    fn remove(&mut self, attr: Bool) {
        self.bool_attrs.remove(attr);
    }
}

impl<Bool, Num, Float> AttrCtx<Float, f64> for Attributes<Bool, Num, Float>
where
    Bool: FromStr + Hash + Eq + Copy,
    Num: FromStr + Hash + Eq + Copy,
    Float: FromStr + Hash + Eq + Copy,
{
    fn get(&self, attr: Float) -> Option<&f64> {
        self.float_attrs.get(attr)
    }

    fn get_loc(&self, attr: Float) -> Option<GPosIdx> {
        self.float_attrs.get_loc(attr)
    }

    fn set(&mut self, attr: Float, value: f64, loc: GPosIdx) {
        self.float_attrs.set(attr, value, loc);
    }

    fn remove(&mut self, attr: Float) {
        self.float_attrs.remove(attr);
    }
}

impl<Bool, Num, Float> Default for Attributes<Bool, Num, Float>
where
    Bool: FromStr + Hash + Eq + Copy,
    Num: FromStr + Hash + Eq + Copy,
    Float: FromStr + Hash + Eq + Copy,
{
    fn default() -> Self {
        Self {
            num_attrs: AttrStore::default(),
            bool_attrs: AttrStore::default(),
            float_attrs: AttrStore::default(),
        }
    }
}
