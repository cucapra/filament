use crate::utils;
use std::marker::PhantomData;

/// A map that stores information of type [Info] about type [Assoc] indexed by type [Key].
///
/// Note: This data structure has no way to track which indices are valid so it
/// is up to the user to ensure that the indices are valid by calling [IndexStore::is_valid]
/// on a valid index.
pub struct DenseIndexInfo<Assoc, Info, Key = utils::Idx<Assoc>>
where
    Key: utils::IdxLike<Assoc>,
{
    store: Vec<Info>,
    _key_typ: PhantomData<Assoc>,
    _idx_typ: PhantomData<Key>,
}

impl<T, V, I> DenseIndexInfo<T, V, I>
where
    I: utils::IdxLike<T>,
{
    /// Construct a new info map with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            store: Vec::with_capacity(cap),
            _key_typ: PhantomData,
            _idx_typ: PhantomData,
        }
    }

    /// Remove all values from the map.
    pub fn clear(&mut self) {
        self.store.clear();
    }

    /// Add a new value to the map and return the index.
    /// Panics if the index is not the next index in the sequence.
    pub fn push(&mut self, key: I, val: V) {
        assert!(self.store.len() == key.get());
        self.store.push(val);
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: I) -> &V {
        &self.store[idx.get()]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: I) -> &mut V {
        &mut self.store[idx.get()]
    }

    /// Check if the map contains the given index.
    pub fn contains(&self, idx: I) -> bool {
        idx.get() < self.store.len()
    }

    /// Get the value associated with the index if present, otherwise return None.
    pub fn find(&self, idx: I) -> Option<&V> {
        if self.contains(idx) {
            Some(self.get(idx))
        } else {
            None
        }
    }

    /// Iterator over the values in the map
    pub fn iter(&self) -> impl Iterator<Item = (I, &V)> + '_ {
        self.store
            .iter()
            .enumerate()
            .map(|(idx, val)| (I::new(idx), val))
    }
}

impl<T, V: Default, I: utils::IdxLike<T>> DenseIndexInfo<T, V, I> {
    /// Extract the value at a particular index and replace it with the default value.
    pub fn take(&mut self, key: I) -> Option<V> {
        if self.store.len() > key.get() {
            // idx is already in the store, take it
            Some(std::mem::take(self.get_mut(key)))
        } else {
            None
        }
    }
}

impl<T, V: Default + Clone, I: utils::IdxLike<T>> DenseIndexInfo<T, V, I> {
    /// Add the value to the map if the index is not already present.
    /// Unlike [Self::push], this method can add values in any order.
    pub fn insert(&mut self, key: I, mut val: V) -> Option<V> {
        if self.store.len() > key.get() {
            // idx is already in the store, need to update it
            std::mem::swap(self.get_mut(key), &mut val);
            Some(val)
        } else {
            self.store.resize(key.get(), V::default());
            self.push(key, val);
            None
        }
    }

    /// Construct a new info map with the given capacity with the default value for each index.
    pub fn with_default(cap: usize) -> Self {
        Self {
            store: vec![V::default(); cap],
            _key_typ: PhantomData,
            _idx_typ: PhantomData,
        }
    }
}

impl<T, V, Idx: utils::IdxLike<T>> FromIterator<(Idx, V)>
    for DenseIndexInfo<T, V, Idx>
{
    fn from_iter<Iter: IntoIterator<Item = (Idx, V)>>(iter: Iter) -> Self {
        let mut store = Self::default();
        for (idx, val) in iter {
            store.push(idx, val);
        }
        store
    }
}

impl<T, V, I: utils::IdxLike<T>> Default for DenseIndexInfo<T, V, I> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            _key_typ: PhantomData,
            _idx_typ: PhantomData,
        }
    }
}

impl<T, V, I> Clone for DenseIndexInfo<T, V, I>
where
    V: Clone,
    I: utils::IdxLike<T>,
{
    fn clone(&self) -> Self {
        Self {
            store: self.store.clone(),
            _key_typ: PhantomData,
            _idx_typ: PhantomData,
        }
    }
}

impl<T, V, I: utils::IdxLike<T>> std::ops::Index<I>
    for DenseIndexInfo<T, V, I>
{
    type Output = V;

    fn index(&self, idx: I) -> &Self::Output {
        self.get(idx)
    }
}
