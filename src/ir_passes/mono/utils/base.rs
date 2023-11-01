use fil_ir::{Idx, IdxLike};

/// Wrap something as a base. This is only implemented for [Idx].
pub trait IntoBase<T> {
    /// Convert this into a Base<T> wrapper.
    fn base(self) -> Base<T>;
}

impl<T> IntoBase<T> for Idx<T> {
    fn base(self) -> Base<T> {
        Base::new(self)
    }
}

#[derive(Debug)]
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
    fn new(idx: Idx<T>) -> Self {
        Self { idx }
    }

    pub fn get(&self) -> Idx<T> {
        self.idx
    }
}

impl<T> Eq for Base<T> {}
impl<T> PartialEq for Base<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}
impl<T> std::hash::Hash for Base<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}
impl<T> Clone for Base<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> PartialOrd for Base<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Base<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}
impl<T> Default for Base<T> {
    fn default() -> Self {
        Self { idx: Idx::UNKNOWN }
    }
}

impl<T> Copy for Base<T> {}
impl<T> IdxLike<T> for Base<T> {
    const UNKNOWN: Self = Self { idx: Idx::UNKNOWN };

    fn new(idx: usize) -> Self {
        Self { idx: Idx::new(idx) }
    }

    fn get(self) -> usize {
        self.idx.get()
    }
}
