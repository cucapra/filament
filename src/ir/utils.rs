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

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &T {
        &self.store[idx.get()]
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> {
        self.store
            .iter()
            .enumerate()
            .map(|(idx, ptr)| (Idx::new(idx), &**ptr))
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

/// An indexed store for a type. Unlike [Interned], this data structure does not deduplicate values.
pub struct IndexStore<T> {
    store: Vec<T>,
}

impl<T> Default for IndexStore<T> {
    fn default() -> Self {
        Self { store: Vec::new() }
    }
}

impl<T> IndexStore<T> {
    /// A a value to the store and return the index.
    pub fn add(&mut self, val: T) -> Idx<T> {
        // Add the value to the store and return index
        let idx = Idx::new(self.store.len());
        self.store.push(val);
        idx
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &T {
        &self.store[idx.get()]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: Idx<T>) -> &mut T {
        &mut self.store[idx.get()]
    }

    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> {
        self.store
            .iter()
            .enumerate()
            .map(|(idx, val)| (Idx::new(idx), val))
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Idx<T>, &mut T)> {
        self.store
            .iter_mut()
            .enumerate()
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

/// A context for storing values with their indices.
/// In addition to adding and getting values, the context also supports applying
/// a substitution to a value.
pub trait Ctx<T> {
    /// Add a new, interned value to the context
    fn add(&mut self, val: T) -> Idx<T>;
    /// Get the information associated with a value
    fn get(&self, idx: Idx<T>) -> &T;
}

impl<T> Ctx<T> for Interned<T>
where
    T: Eq + std::hash::Hash,
{
    fn add(&mut self, val: T) -> Idx<T> {
        self.intern(val)
    }

    fn get(&self, idx: Idx<T>) -> &T {
        self.get(idx)
    }
}

impl<T> Ctx<T> for IndexStore<T> {
    fn add(&mut self, val: T) -> Idx<T> {
        self.add(val)
    }

    fn get(&self, idx: Idx<T>) -> &T {
        self.get(idx)
    }
}

/// A map that stores information of type [V] and is indexed by
/// [Idx<T>] types.
///
/// This is essentially a type-safe way of storing information about value of type [T].
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
}
