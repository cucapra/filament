use crate::{ast::Id, utils::Idx};
use itertools::Itertools;
use std::{collections::HashMap, fmt};

/// A value stored in a particular scope level of a [ScopeMap]
pub struct ScopeVal<V, K = Id> {
    /// The scope-level with which this value is ascoiated
    scope_level: u32,
    name: K,
    val: Idx<V>,
}

impl<V, K> ScopeVal<V, K> {
    pub fn new(scope_level: u32, name: K, val: Idx<V>) -> Self {
        Self {
            scope_level,
            name,
            val,
        }
    }
}

/// Structure to track name bindings through scopes
//
// TODO(rachit): This can be redesigned to be a simple, arena-style vector along with another one to track the scope levels
// The push command would correspond to storing the current length of the vector and the pop command would correspond to
// truncating the vector to the stored length.
// Search for a name would involve reverse iteration.
// Since most Scopes are small, this should be faster than the current implementation.
pub struct ScopeMap<V, K = Id>
where
    K: Eq + std::hash::Hash + Clone,
{
    /// Unique identifier for each scope level
    scope_level: u32,
    map: Vec<HashMap<K, Idx<V>>>,
}
impl<V, K> ScopeMap<V, K>
where
    K: Eq + std::hash::Hash + Clone,
{
    pub fn new() -> Self {
        Self {
            scope_level: 0,
            map: vec![HashMap::new()],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.len() == 1 && self.map[0].is_empty()
    }

    /// Reset the scope map to the initial state
    pub fn reset(&mut self) {
        self.scope_level = 0;
        self.map = vec![HashMap::new()];
    }

    /// Return the current scope level
    pub fn scope_level(&self) -> u32 {
        self.scope_level
    }

    #[inline]
    /// Push a new scope level
    pub fn push(&mut self) {
        self.scope_level += 1;
        self.map.push(HashMap::new());
    }

    #[inline]
    /// Pop the last scope level
    pub fn pop(&mut self) {
        self.map.pop();
        assert!(!self.map.is_empty(), "Cannot pop last scope level");
    }

    /// Insert binding into the scope level and return the [ScopeIdx] value for it
    pub fn insert(&mut self, id: K, idx: Idx<V>) -> ScopeVal<V, K> {
        let scope = self.map.last_mut().unwrap();
        if scope.contains_key(&id) {
            panic!("key already in scope map");
        }
        scope.insert(id.clone(), idx);
        ScopeVal {
            scope_level: self.scope_level,
            name: id,
            val: idx,
        }
    }

    /// Return the value by searching through the scope levels
    pub fn get(&self, id: &K) -> Option<&Idx<V>> {
        for scope in self.map.iter().rev() {
            if let Some(val) = scope.get(id) {
                return Some(val);
            }
        }
        None
    }
}

impl<V, K> ScopeMap<V, K>
where
    K: Eq + std::hash::Hash + Clone,
{
    /// Returns a flattened representation of the scope. This should only be used for reporting errors.
    pub(super) fn as_flat_vec(&self) -> Vec<(K, Idx<V>)> {
        self.map
            .iter()
            .flat_map(|map| map.iter().map(move |(id, val)| (id.clone(), *val)))
            .collect_vec()
    }
}

impl<V, K> fmt::Display for ScopeMap<V, K>
where
    Idx<V>: fmt::Display,
    K: Eq + std::hash::Hash + fmt::Display + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (scope, map) in self.map.iter().enumerate() {
            writeln!(f, "Scope {}:", scope)?;
            if map.is_empty() {
                writeln!(f, "  <empty>")?;
                continue;
            }
            for (id, val) in map.iter() {
                writeln!(f, "  {}={}", id, val)?;
            }
        }
        Ok(())
    }
}

impl<V, K> std::ops::Index<&K> for ScopeMap<V, K>
where
    K: Eq + std::hash::Hash + Clone + std::fmt::Display,
{
    type Output = Idx<V>;

    fn index(&self, id: &K) -> &Self::Output {
        self.get(id).unwrap()
    }
}
