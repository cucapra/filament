use crate::{
    ir::{self, Ctx},
    utils::Idx,
};

pub struct UnderlyingComp<'a>(&'a ir::Component);

trait UnderlyingCtx<T> {
    fn get(&self, k: Underlying<T>) -> &T;
}

impl<'a, T> UnderlyingCtx<T> for UnderlyingComp<'a>
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Underlying<T>) -> &T {
        self.0.get(k.idx())
    }
}

pub struct BaseComp(ir::Component);

trait BaseCtx<T> {
    fn get(&self, k: Base<T>) -> &T;
    fn add(&mut self, val: T) -> Base<T>;
}

impl<T> BaseCtx<T> for BaseComp
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Base<T>) -> &T {
        self.0.get(k.idx())
    }

    fn add(&mut self, val: T) -> Base<T> {
        Base::new(self.0.add(val))
    }
}

/// Wraps an Idx that is meaningful in the base component, which are the new components
/// that we build during monomorphization. As we visit parts of the underlying (pre-mono)
/// component, we need to build new monomorphized structures and add them to the new component.
/// In many cases, we keep these new Idxs around (to keep track of what's been monomorphized
/// already, as well as to maintain information between ports/invokes, for example). The point
/// of this wrapper is to make it clear that this Idx is meaningful only in the new component
/// we are constructing.
pub struct Base<T> {
    idx: Idx<T>,
}

impl<T> Base<T> {
    pub fn new(idx: Idx<T>) -> Self {
        Self { idx }
    }

    pub fn idx(&self) -> Idx<T> {
        self.idx
    }
}

impl<T> Eq for Base<T> {}
impl<T> PartialEq for Base<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx() == other.idx()
    }
}
impl<T> std::hash::Hash for Base<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}

impl<T> Clone for Base<T> {
    fn clone(&self) -> Self {
        Self { idx: self.idx }
    }
}

impl<T> Copy for Base<T> {}

/// Wraps an Idx that is meaningful in the underlying component, which are the existing pre-monomorphization
/// components. These Idxs get passed around between a lot of functions and mappings during monomorphization,
/// so it becomes hard to keep track of which Idx belongs where. This wrapper makes it distinct from base Idxs.
pub struct Underlying<T> {
    idx: Idx<T>,
}

impl<T> Underlying<T> {
    pub fn new(idx: Idx<T>) -> Self {
        Self { idx }
    }

    pub fn idx(&self) -> Idx<T> {
        self.idx
    }
}

impl<T> Eq for Underlying<T> {}
impl<T> PartialEq for Underlying<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx() == other.idx()
    }
}
impl<T> std::hash::Hash for Underlying<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}

impl<T> Clone for Underlying<T> {
    fn clone(&self) -> Self {
        Self { idx: self.idx }
    }
}
impl<T> Copy for Underlying<T> {}
