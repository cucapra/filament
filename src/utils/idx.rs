use std::marker::PhantomData;

#[derive(Eq, Debug)]
/// Wrapper around a newtyped index associated with a type-level tag.
/// Since the type does not contain a value of type T, it is always copy.
pub struct Idx<T> {
    idx: u32,
    _phantom: PhantomData<T>,
}

impl<T> PartialEq for Idx<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx == other.idx
    }
}

impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        Self {
            idx: self.idx,
            _phantom: PhantomData,
        }
    }
}

/// All indexes are copy
impl<T> Copy for Idx<T> {}

impl<T> Idx<T> {
    /// Representing an unknown index
    pub const UNKNOWN: Self = Self {
        idx: u32::MAX,
        _phantom: PhantomData,
    };

    /// Create a new index
    pub fn new(idx: usize) -> Self {
        Self {
            idx: idx as u32,
            _phantom: PhantomData,
        }
    }

    /// Get the index
    pub fn get(self) -> usize {
        debug_assert!(
            self.idx != u32::MAX,
            "Attempting to convert unknown index"
        );
        self.idx as usize
    }
}
