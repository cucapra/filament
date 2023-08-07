use crate::utils::Idx;

/// Wraps an Idx that is meaningful in the base component
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

/// Wraps an Idx that is meaningful in the underlying component
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
