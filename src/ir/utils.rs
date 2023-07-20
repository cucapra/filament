use bitvec::vec::BitVec;

use super::Ctx;
use crate::utils::Idx;
use std::{collections::HashMap, fmt::Display, marker::PhantomData, rc::Rc};

/// An indexed storage for an interned type. Keeps a HashMap to provide faster reverse mapping
/// from the value to the index.
/// Useful for types that are added continuously throughout the compiler's execution.
///
/// ## Safety
/// The data structure internally stores a pointer to each value stored. This is safe
/// because we do not allow deletion of keys.
/// If a key is ever deleted, a call to `get` will return a dangling pointer.
pub struct Interned<T>
where
    T: Eq + std::hash::Hash,
{
    store: Vec<Rc<T>>,
    map: HashMap<Rc<T>, Idx<T>>,
}

impl<T> Default for Interned<T>
where
    T: Eq + std::hash::Hash,
{
    fn default() -> Self {
        Self {
            store: Vec::new(),
            map: HashMap::new(),
        }
    }
}

impl<T> Interned<T>
where
    T: Eq + std::hash::Hash,
{
    /// Intern a value into the store and return the index.
    /// If the value is already in the store, return the existing index.
    pub fn intern(&mut self, val: T) -> Idx<T> {
        let v = Rc::new(val);
        if let Some(idx) = self.map.get(&v) {
            return *idx;
        }
        // Otherwise, add the value to the store and map
        let idx = Idx::new(self.store.len());
        self.store.push(v.clone());
        self.map.insert(v, idx);
        idx
    }

    /// Number of interned values in the store.
    pub fn size(&self) -> usize {
        self.store.len()
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &T {
        &self.store[idx.get()]
    }

    /// Iterator over the interned values.
    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> {
        self.store
            .iter()
            .enumerate()
            .map(|(idx, ptr)| (Idx::new(idx), &**ptr))
    }

    /// Iterator over indices of the interned values.
    /// Useful since it does not take ownership of self.
    pub fn idx_iter(&self) -> impl Iterator<Item = Idx<T>> {
        (0..self.store.len()).map(Idx::new)
    }
}

impl<T> Display for Interned<T>
where
    T: Eq + std::hash::Hash + Display,
    Idx<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, val) in self.iter() {
            writeln!(f, "{idx}={}", *val)?;
        }
        Ok(())
    }
}

/// An indexed store for a type.
/// Unlike [Interned], this data structure does not deduplicate values and supports mutation of values and removal of indices.
pub struct IndexStore<T> {
    store: Vec<T>,
    /// Tracks which indices have been marked invalid.
    valid: BitVec,
}

impl<T> Default for IndexStore<T> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            valid: BitVec::new(),
        }
    }
}

impl<T> IndexStore<T> {
    /// A a value to the store and return the index.
    pub fn add(&mut self, val: T) -> Idx<T> {
        // Add the value to the store and return index
        let idx = Idx::new(self.store.len());
        self.store.push(val);
        self.valid.push(true);
        idx
    }

    /// Mark an index as invalid.
    /// The underlying data structure does not actually deallocate or reuse the value associated with the index.
    pub fn delete(&mut self, idx: Idx<T>) {
        let i = idx.get();
        // Should not attempt to remove an index that has already been removed
        assert!(self.valid[i], "Attempted to delete invalid index {i}.");
        self.valid.set(i, false);
    }

    /// Returns true iff the index is valid.
    pub fn is_valid(&self, idx: Idx<T>) -> bool {
        self.valid[idx.get()]
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &T {
        let i = idx.get();
        assert!(self.valid[i], "Attempted to get invalid index {i}.");
        &self.store[i]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: Idx<T>) -> &mut T {
        let i = idx.get();
        assert!(self.valid[i], "Attempted to get invalid index {i}.");
        &mut self.store[i]
    }

    /// Number of elements in the store
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Iterate over the indices in the store along with their validity.
    /// This can be useful because it allows mutable borrows of the owners of the store.
    pub fn idx_iter(&self) -> impl Iterator<Item = (Idx<T>, bool)> {
        (0..self.store.len())
            .map(Idx::new)
            .zip(self.valid.clone().into_iter())
    }

    /// Iterate over the indices and the values in the store.
    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> + '_ {
        self.store
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.valid[*idx])
            .map(|(idx, val)| (Idx::new(idx), val))
    }

    /// Mutable iteration over the valid indices and the values in the store.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Idx<T>, &mut T)> {
        self.store
            .iter_mut()
            .enumerate()
            .filter(|(idx, _)| self.valid[*idx])
            .map(|(idx, val)| (Idx::new(idx), val))
    }

    pub(super) fn checked_add(&mut self, idx: Idx<T>, val: T) {
        assert!(
            idx.get() == self.store.len(),
            "Attempting to add index {} but next index is {}",
            idx.get(),
            self.store.len()
        );
        self.add(val);
    }
}

impl<T: Display> Display for IndexStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, val) in self.store.iter().enumerate() {
            writeln!(f, "{idx}={val}")?;
        }
        Ok(())
    }
}

/// A map that stores information of type [V] and is indexed by
/// [Idx<T>] types.
///
/// This is essentially a type-safe way of storing information about value of type [T].
///
/// Note: This data structure has no way to track which indices are valid so it
/// is up to the user to ensure that the indices are valid by calling [IndexStore::is_valid]
/// on a valid index.
pub struct DenseIndexInfo<T, V> {
    store: Vec<V>,
    key_typ: PhantomData<T>,
}

impl<T, V> Default for DenseIndexInfo<T, V> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            key_typ: PhantomData,
        }
    }
}

impl<T, V> DenseIndexInfo<T, V> {
    /// Construct a new info map with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            store: Vec::with_capacity(cap),
            key_typ: PhantomData,
        }
    }

    /// Remove all values from the map.
    pub fn clear(&mut self) {
        self.store.clear();
    }

    /// Add a new value to the map and return the index.
    /// Panics if the index is not the next index in the sequence.
    pub fn push(&mut self, key: Idx<T>, val: V) {
        assert!(self.store.len() == key.get());
        self.store.push(val);
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &V {
        &self.store[idx.get()]
    }

    /// Check if the map contains the given index.
    pub fn contains(&self, idx: Idx<T>) -> bool {
        idx.get() < self.store.len()
    }
}

impl<T, V> std::ops::Index<Idx<T>> for DenseIndexInfo<T, V> {
    type Output = V;

    fn index(&self, idx: Idx<T>) -> &Self::Output {
        self.get(idx)
    }
}

#[derive(Copy)]
/// A reference to a foreign key and its owner.
/// On its own, a foreign key is not very useful. We need provide it with a context
/// that can resolve the owner which can then resolve the underlying type.
/// However, we do not provide a way to extract the underyling `T`.
pub struct Foreign<T, C> {
    /// A reference to the underlying value.
    key: Idx<T>,
    /// A reference to the owner of the foreign key.
    owner: Idx<C>,
}

impl<T, C> Foreign<T, C> {
    pub fn new(key: Idx<T>, owner: Idx<C>) -> Self {
        Self { key, owner }
    }

    /// Map over the foreign key using the given context.
    /// We require a context to resolve the owner of the foreign key.
    pub fn map<X, F>(&self, mut f: F, ctx: impl Ctx<C>) -> Foreign<X, C>
    where
        F: FnMut(Idx<T>, &C) -> Idx<X>,
    {
        let c_resolved = ctx.get(self.owner);
        Foreign {
            key: f(self.key, c_resolved),
            owner: self.owner,
        }
    }
}

impl<T, C> Clone for Foreign<T, C> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            owner: self.owner,
        }
    }
}

impl<T, C> PartialEq for Foreign<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.owner == other.owner
    }
}

impl<T, C> Eq for Foreign<T, C> {}

impl<T, C> std::hash::Hash for Foreign<T, C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.owner.hash(state);
    }
}
