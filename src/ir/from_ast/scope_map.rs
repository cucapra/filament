use crate::{ast::Id, utils::Idx};
use itertools::Itertools;
use std::{collections::HashMap, fmt};

/// Structure to track name bindings through scopes
pub struct ScopeMap<V, K = Id>
where
    K: Eq + std::hash::Hash,
{
    map: Vec<HashMap<K, Idx<V>>>,
}
impl<V, K> ScopeMap<V, K>
where
    K: Eq + std::hash::Hash,
{
    pub fn new() -> Self {
        Self {
            map: vec![HashMap::new()],
        }
    }

    #[inline]
    /// Push a new scope level
    pub fn push(&mut self) {
        self.map.push(HashMap::new());
    }

    #[inline]
    /// Pop the last scope level
    pub fn pop(&mut self) {
        self.map.pop();
        assert!(!self.map.is_empty(), "Cannot pop last scope level");
    }

    /// Insert binding into the scope level
    pub fn insert(&mut self, id: K, idx: Idx<V>) {
        self.map.last_mut().unwrap().insert(id, idx);
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
    K: Eq + std::hash::Hash + fmt::Display,
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
    K: Eq + std::hash::Hash,
{
    type Output = Idx<V>;

    fn index(&self, id: &K) -> &Self::Output {
        self.get(id).unwrap()
    }
}
