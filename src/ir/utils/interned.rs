use crate::{
    ir::{AddCtx, Ctx},
    utils::{self, Idx},
};
use std::{collections::HashMap, fmt::Display, rc::Rc};

#[derive(Clone)]
/// An indexed storage for an interned type. Keeps a HashMap to provide faster reverse mapping
/// from the value to the index.
/// Useful for types that are added continuously throughout the compiler's execution.
///
/// ## Safety
/// The data structure internally stores a pointer to each value stored. This is safe
/// because we do not allow deletion of keys.
/// If a key is ever deleted, a call to `get` will return a dangling pointer.
pub struct Interned<T, I = utils::Idx<T>>
where
    T: Eq + std::hash::Hash,
    I: utils::IdxLike<T>,
{
    store: Vec<Rc<T>>,
    map: HashMap<Rc<T>, I>,
}

impl<T> Ctx<T> for Interned<T>
where
    T: Eq + std::hash::Hash,
{
    fn get(&self, idx: Idx<T>) -> &T {
        self.get(idx)
    }
}

impl<T> AddCtx<T> for Interned<T>
where
    T: Eq + std::hash::Hash,
{
    fn add(&mut self, val: T) -> Idx<T> {
        self.intern(val)
    }
}

impl<T, I> Default for Interned<T, I>
where
    T: Eq + std::hash::Hash,
    I: utils::IdxLike<T>,
{
    fn default() -> Self {
        Self {
            store: Vec::new(),
            map: HashMap::new(),
        }
    }
}

impl<T, I> Interned<T, I>
where
    T: Eq + std::hash::Hash,
    I: utils::IdxLike<T>,
{
    /// Intern a value into the store and return the index.
    /// If the value is already in the store, return the existing index.
    pub fn intern(&mut self, val: T) -> I {
        let v = Rc::new(val);
        if let Some(idx) = self.map.get(&v) {
            return *idx;
        }
        // Otherwise, add the value to the store and map
        let idx = I::new(self.store.len());
        self.store.push(v.clone());
        self.map.insert(v, idx);
        idx
    }

    /// Return the index of the value if it is in the store
    pub fn find(&self, val: &T) -> Option<I> {
        self.map.get(val).copied()
    }

    /// Number of interned values in the store.
    pub fn size(&self) -> usize {
        self.store.len()
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: I) -> &T {
        &self.store[idx.get()]
    }

    /// Iterator over the interned values.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.store
            .iter()
            .enumerate()
            .map(|(idx, ptr)| (I::new(idx), &**ptr))
    }

    /// Iterator over indices of the interned values.
    /// Useful since it does not take ownership of self.
    pub fn idx_iter(&self) -> impl Iterator<Item = I> {
        (0..self.store.len()).map(I::new)
    }
}

impl<T> Display for Interned<T>
where
    T: Eq + std::hash::Hash + Display,
    utils::Idx<T>: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (idx, val) in self.iter() {
            writeln!(f, "{idx}={}", *val)?;
        }
        Ok(())
    }
}
