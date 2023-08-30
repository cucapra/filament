use super::Component;
use crate::utils::{Idx, IdxLike};

/// A context for storing values with their indices.
/// The context is indexed by [`IdxLike<T>`].
pub trait AddCtx<T, I = Idx<T>>
where
    I: IdxLike<T>,
{
    /// Add a new value to the context
    fn add(&mut self, val: T) -> I;
}

/// A that provides immutable access to values using [`IdxLike<T>`] indices.
pub trait Ctx<T, I = Idx<T>>
where
    I: IdxLike<T>,
{
    /// Get the information associated with a value
    fn get(&self, idx: I) -> &T;
}

/// A context that provides mutable access to values using [`IdxLike<T>`] indices.
pub trait MutCtx<T, I = Idx<T>>
where
    I: IdxLike<T>,
{
    /// Get a mutable reference to the value associated with the index.
    fn get_mut(&mut self, idx: I) -> &mut T;
    /// Delete the value associated with the index.
    fn delete(&mut self, idx: I);
}

/// We can use indexing syntax for all values in the context for which it is a [`Ctx<K>`].
impl<K> std::ops::Index<Idx<K>> for Component
where
    Component: Ctx<K>,
{
    type Output = K;

    fn index(&self, index: Idx<K>) -> &Self::Output {
        self.get(index)
    }
}
