use crate::utils::Idx;
use std::{collections::HashMap, marker::PhantomData};

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
    store: Vec<*const T>,
    map: HashMap<T, Idx<T>>,
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
        if let Some(idx) = self.map.get(&val) {
            return *idx;
        }
        // Otherwise, add the value to the store and map
        let idx = Idx::new(self.store.len());
        self.store.push(&val);
        self.map.insert(val, idx);
        idx
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &T {
        let pointer = self.store[idx.get()];
        unsafe { pointer.as_ref().unwrap() }
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
pub struct DenseIndexInfo<T, V>
where
    V: Default,
{
    store: Vec<V>,
    key_typ: PhantomData<T>,
}

impl<T, V> Default for DenseIndexInfo<T, V>
where
    V: Default,
{
    fn default() -> Self {
        Self {
            store: Vec::new(),
            key_typ: PhantomData,
        }
    }
}

impl<T, V> DenseIndexInfo<T, V>
where
    V: Default,
{
    /// Add a new value to the map and return the index.
    pub fn add(&mut self, key: Idx<T>, val: V) {
        // Resize the store if it is not large enough to hold the key
        let len = self.store.len();
        // if the index is exactly the length of the store, we need to add one
        // more element so use `push` otherwise, we need to reserve space and
        // fill the newly added space with default values
        if len == key.get() {
            self.store.push(val);
            return;
        }

        if len < key.get() {
            self.store.reserve(key.get() + 1);
            let diff = key.get() - len + 1;
            // Fill the newly added space with default values
            for _ in 0..diff {
                self.store.push(V::default());
            }
        }
        self.store[key.get()] = val;
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &V {
        &self.store[idx.get()]
    }
}
