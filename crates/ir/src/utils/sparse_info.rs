use crate::utils::{Idx, IdxLike};
use std::collections::HashMap;

/// A sparse information map to store information associated with indices.
/// Implements the same methods as [super::DenseIndexInfo] but is more efficient
/// when the indices are sparse.
pub struct SparseInfoMap<Assoc, Info, Key = Idx<Assoc>>
where
    Key: IdxLike<Assoc>,
{
    map: HashMap<Key, Info>,
    _key_typ: std::marker::PhantomData<Assoc>,
}

impl<Assoc, Info, Key> SparseInfoMap<Assoc, Info, Key>
where
    Key: IdxLike<Assoc>,
{
    /// Construct a new info map with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity(cap),
            _key_typ: std::marker::PhantomData,
        }
    }

    /// Remove all values from the map.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Add a new value to the map and return the index.
    pub fn push(&mut self, key: Key, val: Info) {
        self.map.insert(key, val);
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Key) -> &Info {
        &self.map[&idx]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: Key) -> &mut Info {
        self.map.get_mut(&idx).unwrap()
    }

    /// Check if the map contains the given index.
    pub fn contains(&self, idx: Key) -> bool {
        self.map.contains_key(&idx)
    }

    /// Get the value associated with the index if present, otherwise return None.
    pub fn find(&self, idx: Key) -> Option<&Info> {
        self.map.get(&idx)
    }

    /// Iterator over the values in the map
    pub fn iter(&self) -> impl Iterator<Item = (Key, &Info)> + '_ {
        self.map.iter().map(|(idx, val)| (*idx, val))
    }

    /// Extend the map with the values from another map.
    pub fn extend(&mut self, other: Self) {
        self.map.extend(other.map);
    }
}

impl<Assoc, Info, Key> Default for SparseInfoMap<Assoc, Info, Key>
where
    Key: IdxLike<Assoc>,
{
    fn default() -> Self {
        Self {
            map: HashMap::new(),
            _key_typ: std::marker::PhantomData,
        }
    }
}

impl<Assoc, Info, Key> Clone for SparseInfoMap<Assoc, Info, Key>
where
    Info: Clone,
    Key: IdxLike<Assoc>,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
            _key_typ: std::marker::PhantomData,
        }
    }
}

impl<T, V, Idx: IdxLike<T>> FromIterator<(Idx, V)>
    for SparseInfoMap<T, V, Idx>
{
    fn from_iter<Iter: IntoIterator<Item = (Idx, V)>>(iter: Iter) -> Self {
        let mut store = Self::default();
        for (idx, val) in iter {
            store.push(idx, val);
        }
        store
    }
}

impl<Assoc, Info, Key> IntoIterator for SparseInfoMap<Assoc, Info, Key>
where
    Key: IdxLike<Assoc>,
{
    type Item = (Key, Info);
    type IntoIter = std::collections::hash_map::IntoIter<Key, Info>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<T, V, Idx: IdxLike<T>> std::ops::Index<Idx> for SparseInfoMap<T, V, Idx> {
    type Output = V;

    fn index(&self, index: Idx) -> &Self::Output {
        self.get(index)
    }
}
