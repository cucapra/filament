use super::Component;
use crate::{utils::Idx};

/// A context for storing values with their indices.
/// The context is indexed by [Idx<T>].
pub trait Ctx<T> {
    /// Add a new value to the context
    fn add(&mut self, val: T) -> Idx<T>;
    /// Get the information associated with a value
    fn get(&self, idx: Idx<T>) -> &T;
}

/// A context that provides mutable access to values using [Idx<T>] indices.
pub trait MutCtx<T> {
    /// Get a mutable reference to the value associated with the index.
    fn get_mut(&mut self, idx: Idx<T>) -> &mut T;
}

// We can use indexing syntax for all values in the context for which it is a Ctx.
impl<K> std::ops::Index<Idx<K>> for Component
where
    Component: Ctx<K>,
{
    type Output = K;

    fn index(&self, index: Idx<K>) -> &Self::Output {
        self.get(index)
    }
}
