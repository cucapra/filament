use fil_ir::{Idx, IdxLike};

/// Wrap something as a Underlying. This is only implemented for [Idx].
pub trait IntoUdl<T> {
    fn ul(self) -> Underlying<T>;
}

impl<T> IntoUdl<T> for Idx<T> {
    fn ul(self) -> Underlying<T> {
        Underlying::new(self)
    }
}

#[derive(Debug)]
/// Wraps an Idx that is meaningful in the underlying component, which are the existing pre-monomorphization
/// components. These Idxs get passed around between a lot of functions and mappings during monomorphization,
/// so it becomes hard to keep track of which Idx belongs where. This wrapper makes it distinct from base Idxs.
pub struct Underlying<T> {
    idx: Idx<T>,
}

impl<T> Underlying<T> {
    fn new(idx: Idx<T>) -> Self {
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
        *self
    }
}
impl<T> Copy for Underlying<T> {}
impl<T> PartialOrd for Underlying<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Underlying<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

impl<T> IdxLike<T> for Underlying<T> {
    const UNKNOWN: Self = Self { idx: Idx::UNKNOWN };

    fn new(idx: usize) -> Self {
        Self { idx: Idx::new(idx) }
    }

    fn get(self) -> usize {
        self.idx.get()
    }
}
