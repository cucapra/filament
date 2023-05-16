use crate::ast::Id;
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::{fmt::Debug, fmt::Display};

#[derive(Default, Clone)]
/// Represents a binding from names to type T.
pub struct Binding<T> {
    map: LinkedHashMap<Id, T>,
}

impl<T> Binding<T> {
    pub fn new(map: impl IntoIterator<Item = (Id, T)>) -> Self {
        Self {
            map: map.into_iter().collect(),
        }
    }

    /// Insert a binding
    pub fn insert(&mut self, n: Id, t: T) {
        self.map.insert(n, t);
    }

    /// Find the binding for n, or return None
    pub fn find(&self, n: &Id) -> Option<&T> {
        self.map.get(n)
    }

    /// Return binding for n, or panic if it doesn't exist
    pub fn get(&self, n: &Id) -> &T {
        self.find(n).unwrap_or_else(|| panic!("No binding for {n}"))
    }

    /// Iterate over the values of the binding
    pub fn values(&self) -> impl Iterator<Item = &T> {
        self.map.values()
    }

    /// Iterate over the binding
    pub fn iter(&self) -> impl Iterator<Item = (&Id, &T)> {
        self.map.iter()
    }

    /// Extend the binding
    pub fn extend(&mut self, other: Vec<(Id, T)>) {
        self.map.extend(other);
    }

    /// Check if the binding is emtpy
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

impl<T> std::ops::Index<&Id> for Binding<T> {
    type Output = T;

    fn index(&self, n: &Id) -> &Self::Output {
        self.get(n)
    }
}

impl<T> IntoIterator for Binding<T> {
    type Item = (Id, T);
    type IntoIter = linked_hash_map::IntoIter<Id, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.map.into_iter()
    }
}

impl<T> Debug for Binding<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "[{}]",
            self.map.iter().map(|(k, v)| format!("{k}->{v}")).join(", ")
        )
    }
}
