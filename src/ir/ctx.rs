use super::Component;
use crate::utils::{Idx, IdxLike};

/// A context for storing values with their indices.
/// The context is indexed by [`Idx<T>`].
pub trait Ctx<T, K = Idx<T>>
where
    K: IdxLike<T>,
{
    /// Add a new value to the context
    fn add(&mut self, val: T) -> K;
    /// Get the information associated with a value
    fn get(&self, idx: K) -> &T;
}

/// A context that provides mutable access to values using [`Idx<T>`] indices.
pub trait MutCtx<T, K = Idx<T>>
where
    K: IdxLike<T>,
{
    /// Get a mutable reference to the value associated with the index.
    fn get_mut(&mut self, idx: K) -> &mut T;
    /// Delete the value associated with the index.
    fn delete(&mut self, idx: K);
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
