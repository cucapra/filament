use std::{marker::PhantomData, num::NonZeroU32};

#[macro_export]
macro_rules! idx {
    ($name:ty) => {
       $crate::utils::Idx<$name>
    };
}

#[derive(Eq, Debug)]
/// Wrapper around a newtyped index associated with a type-level tag.
/// Since the type does not contain a value of type T, it is always copy.
pub struct Idx<T> {
    idx: NonZeroU32,
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
        idx: unsafe { NonZeroU32::new_unchecked(u32::MAX - 1) },
        _phantom: PhantomData,
    };

    /// Create a new index
    pub fn new(idx: usize) -> Self {
        Self {
            idx: NonZeroU32::new(u32::try_from(idx + 1).unwrap()).unwrap(),
            _phantom: PhantomData,
        }
    }

    /// Get the index
    pub fn get(self) -> usize {
        debug_assert!(
            self != Self::UNKNOWN,
            "attempting to convert unknown index for type `{}'",
            std::any::type_name::<T>(),
        );
        (self.idx.get() - 1) as usize
    }
}
