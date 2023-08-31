use std::{marker::PhantomData, num::NonZeroU32};

#[macro_export]
macro_rules! idx {
    ($name:ty) => {
       $crate::utils::Idx<$name>
    };
}

#[macro_export]
macro_rules! define_idx {
    ($name:ident, $st:ty, $pre:expr) => {
        pub type $name = $crate::utils::Idx<$st>;
        impl ::std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                // We love MLIR
                write!(f, concat!("%", $pre, "{}"), self.get())
            }
        }
        impl ::std::fmt::Debug for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self)
            }
        }
    };
}

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

impl<T> Eq for Idx<T> {}

impl<T> std::hash::Hash for Idx<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}

impl<T> Clone for Idx<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialOrd for Idx<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx.partial_cmp(&other.idx)
    }
}

impl<T> Ord for Idx<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
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

    #[inline]
    /// Create a new index
    pub fn new(idx: usize) -> Self {
        Self {
            idx: NonZeroU32::new(u32::try_from(idx + 1).unwrap()).unwrap(),
            _phantom: PhantomData,
        }
    }

    #[inline]
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

/// A type that can act like an `Idx`
pub trait IdxLike<T>
where
    Self: Copy + std::hash::Hash + Eq + Ord + Sized,
{
    const UNKNOWN: Self;
    /// Create a new index using a usize
    fn new(idx: usize) -> Self;
    /// Get the index underlying the type
    fn get(self) -> usize;
}

impl<T> IdxLike<T> for Idx<T> {
    const UNKNOWN: Self = Self::UNKNOWN;
    fn new(idx: usize) -> Self {
        Self::new(idx)
    }
    fn get(self) -> usize {
        self.get()
    }
}
