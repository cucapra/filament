use crate::utils::{Idx, IdxLike};
use std::collections::HashMap;

#[derive(Clone, Default)]
/// A sparse information map to store information associated with indices.
/// Implements the same methods as [super::DenseIndexInfo] but is more efficient
/// when the indices are sparse.
pub struct SparseInfoMap<V, Key = Idx<V>>
where
    Key: IdxLike<V>,
{
    map: HashMap<Key, V>,
}

impl<V, K> SparseInfoMap<V, K>
where
    K: IdxLike<V>,
{
    /// Construct a new info map with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            map: HashMap::with_capacity(cap),
        }
    }

    /// Remove all values from the map.
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Add a new value to the map and return the index.
    /// Panics if the index is not the next index in the sequence.
    pub fn push(&mut self, key: K, val: V) {
        self.map.insert(key, val);
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: K) -> &V {
        &self.map[&idx]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: K) -> &mut V {
        self.map.get_mut(&idx).unwrap()
    }

    /// Check if the map contains the given index.
    pub fn contains(&self, idx: K) -> bool {
        self.map.contains_key(&idx)
    }

    /// Get the value associated with the index if present, otherwise return None.
    pub fn find(&self, idx: K) -> Option<&V> {
        self.map.get(&idx)
    }

    /// Iterator over the values in the map
    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> + '_ {
        self.map.iter().map(|(idx, val)| (*idx, val))
    }
}
