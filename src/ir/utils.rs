use crate::utils::Idx;
use std::collections::HashMap;

/// An indexed storage for an interned type. Keeps a HashMap to provide faster reverse mapping
/// from the value to the index.
/// Useful for types that are added continuously throughout the compiler's execution.
///
/// ## Safety
/// The data structure internally stores a pointer to each value stored. This is safe
/// because we do not allow deletion of keys.
/// If a key is ever deleted, a call to `get` will return a dangling pointer.
pub struct Indexed<T>
where
    T: Eq + std::hash::Hash,
{
    store: Vec<*const T>,
    map: HashMap<T, Idx<T>>,
}

impl<T> Indexed<T>
where
    T: Eq + std::hash::Hash,
{
    pub fn add(&mut self, val: T) -> Idx<T> {
        // If the value is already in the map, return the index
        if let Some(idx) = self.map.get(&val) {
            return *idx;
        }
        // Otherwise, add the value to the store and map
        let idx = Idx::new(self.store.len());
        self.store.push(&val);
        self.map.insert(val, idx);
        idx
    }

    pub fn get(&self, idx: Idx<T>) -> &T {
        let pointer = self.store[idx.get()];
        unsafe { pointer.as_ref().unwrap() }
    }
}

/// A small indexed storage for type. Prefer this over [Indexed] when we
/// don't expect the number of elements to be large.
/// Adding new elements is an O(n) operation since we search the store for the
/// element first.
pub struct SmallIndexed<T>
where
    T: Eq,
{
    store: Vec<T>,
}

impl<T> SmallIndexed<T>
where
    T: Eq,
{
    pub fn add(&mut self, val: T) -> Idx<T> {
        // If the value is already in the map, return the index
        if let Some(idx) = self.store.iter().position(|v| *v == val) {
            return Idx::new(idx);
        }
        // Otherwise, add the value to the store and map
        let idx = Idx::new(self.store.len());
        self.store.push(val);
        idx
    }

    pub fn get(&self, idx: Idx<T>) -> &T {
        &self.store[idx.get()]
    }
}

/// A context for interning values.
/// In addition to adding and getting values, the context also supports applying
/// a substitution to a value.
pub trait Ctx<T> {
    /// Add a value to the context
    fn add(&mut self, val: T) -> Idx<T>;
    /// Get the information associated with a value
    fn get(&self, idx: Idx<T>) -> &T;
}
