use crate::{AddCtx, Ctx, MutCtx, utils, utils::Idx};
use bitvec::vec::BitVec;
use std::{fmt::Display, marker::PhantomData};

#[derive(Clone)]
/// An indexed store for a type.
/// Unlike [Interned], this data structure does not deduplicate values and supports mutation of values and removal of indices.
pub struct IndexStore<T, I = utils::Idx<T>>
where
    I: utils::IdxLike<T>,
{
    store: Vec<T>,
    /// Tracks which indices have been marked invalid.
    valid: BitVec,
    /// The kind of index used to access the store.
    _type: PhantomData<I>,
}

impl<T> Default for IndexStore<T> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            valid: BitVec::new(),
            _type: PhantomData,
        }
    }
}

impl<T, I> IndexStore<T, I>
where
    I: utils::IdxLike<T>,
{
    /// A a value to the store and return the index.
    pub fn add(&mut self, val: T) -> I {
        // Add the value to the store and return index
        let idx = I::new(self.store.len());
        self.store.push(val);
        self.valid.push(true);
        idx
    }

    /// Mark an index as invalid.
    /// The underlying data structure does not actually deallocate or reuse the value associated with the index.
    pub fn delete(&mut self, idx: I) {
        let i = idx.get();
        // Should not attempt to remove an index that has already been removed
        assert!(self.valid[i], "Attempted to delete invalid index {i}.");
        self.valid.set(i, false);
    }

    /// Returns true iff the index is valid.
    pub fn is_valid(&self, idx: I) -> bool {
        self.valid[idx.get()]
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: I) -> &T {
        let i = idx.get();
        assert!(
            self.store.len() > i,
            "Attempted to get index {i} but store only has {} elements.",
            self.store.len()
        );
        assert!(self.valid[i], "Attempted to get invalid index {i}.");
        &self.store[i]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: I) -> &mut T {
        let i = idx.get();
        assert!(
            self.store.len() > i,
            "Attempted to get index {i} but store only has {} elements.",
            self.store.len()
        );
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

    /// Iterate over the valid indices in the store.
    /// This can be useful because it allows mutable borrows of the owners of the store.
    pub fn idx_iter(&self) -> impl Iterator<Item = I> + use<T, I> {
        self.valid
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(_, valid)| *valid)
            .map(|(idx, _)| I::new(idx))
    }

    /// Iterate over the valid indices and the values in the store.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.store
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.valid[*idx])
            .map(|(idx, val)| (I::new(idx), val))
    }

    /// Mutable iteration over the valid indices and the values in the store.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> {
        self.store
            .iter_mut()
            .enumerate()
            .filter(|(idx, _)| self.valid[*idx])
            .map(|(idx, val)| (I::new(idx), val))
    }

    /// Add a value to a specific index.
    /// Panics if the index does not already contain all the previous value.
    pub fn checked_add(&mut self, idx: I, val: T) {
        assert!(
            idx.get() == self.store.len(),
            "Attempting to add index {} but next index is {}",
            idx.get(),
            self.store.len()
        );
        self.add(val);
    }
}

impl<T> From<IndexStore<T>> for Vec<T> {
    fn from(value: IndexStore<T>) -> Self {
        value.store
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

impl<T> Ctx<T> for IndexStore<T> {
    fn get(&self, idx: Idx<T>) -> &T {
        self.get(idx)
    }
}

impl<T> AddCtx<T> for IndexStore<T> {
    fn add(&mut self, val: T) -> Idx<T> {
        self.add(val)
    }
}

impl<T> MutCtx<T> for IndexStore<T> {
    fn get_mut(&mut self, idx: Idx<T>) -> &mut T {
        self.get_mut(idx)
    }

    fn delete(&mut self, idx: Idx<T>) {
        self.delete(idx)
    }

    fn valid(&self, idx: Idx<T>) -> bool {
        self.is_valid(idx)
    }
}
