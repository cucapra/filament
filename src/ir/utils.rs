use itertools::Itertools;
use topological_sort::TopologicalSort;

use super::Ctx;
use crate::utils::Idx;
use std::{
    collections::HashMap, fmt::Display, iter::IntoIterator,
    marker::PhantomData, rc::Rc,
};

use super::{Command, CompIdx, Context};

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

    /// Number of interned values in the store.
    pub fn size(&self) -> usize {
        self.store.len()
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: Idx<T>) -> &T {
        &self.store[idx.get()]
    }

    /// Iterator over the interned values.
    pub fn iter(&self) -> impl Iterator<Item = (Idx<T>, &T)> {
        self.store
            .iter()
            .enumerate()
            .map(|(idx, ptr)| (Idx::new(idx), &**ptr))
    }

    /// Iterator over indices of the interned values.
    /// Useful since it does not take ownership of self.
    pub fn idx_iter(&self) -> impl Iterator<Item = Idx<T>> {
        (0..self.store.len()).map(Idx::new)
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

    /// Number of elements in the store
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Iterate over the indices in the store.
    /// This can be useful because it allows us to mutably borrow the containing struct.
    pub fn idx_iter(&self) -> impl Iterator<Item = Idx<T>> {
        (0..self.store.len()).map(Idx::new)
    }

    /// Iterate over the indices and the values in the store.
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

impl<T> IntoIterator for IndexStore<T> {
    type Item = T;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.store.into_iter()
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
    /// Construct a new info map with the given capacity.
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            store: Vec::with_capacity(cap),
            key_typ: PhantomData,
        }
    }

    /// Remove all values from the map.
    pub fn clear(&mut self) {
        self.store.clear();
    }

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

    /// Check if the map contains the given index.
    pub fn contains(&self, idx: Idx<T>) -> bool {
        idx.get() < self.store.len()
    }
}

impl<T, V> std::ops::Index<Idx<T>> for DenseIndexInfo<T, V> {
    type Output = V;

    fn index(&self, idx: Idx<T>) -> &Self::Output {
        self.get(idx)
    }
}

pub struct Traversal {
    ctx: Context,
    order: Vec<CompIdx>,
}

impl From<Context> for Traversal {
    /// Construct a post-order traversal over a [Context].
    fn from(ctx: Context) -> Self {
        let mut ts = TopologicalSort::<CompIdx>::new();

        // Gets all components that are not primitives.
        let comps = ctx
            .comps
            .iter()
            .filter(|(_, comp)| !comp.is_ext)
            .collect_vec();

        for (idx, _) in comps.iter() {
            ts.insert(*idx);
        }

        for (idx, comp) in comps.iter() {
            for cmd in &comp.cmds {
                Traversal::process_cmd(&ctx, *idx, cmd, &mut ts);
            }
        }

        let order: Vec<_> = ts.collect();
        debug_assert!(
            order.len() == comps.len(),
            "Ordering contains {} elements but context has {} components",
            order.len(),
            comps.len()
        );

        Self { ctx, order }
    }
}

impl Traversal {
    /// Apply a function to each component in a post-order traversal.
    pub fn apply_post_order<F>(self, mut f: F)
    where
        F: FnMut(&Context, CompIdx),
    {
        for idx in self.order.clone() {
            log::trace!("Post-order: {}", idx);
            f(&self.ctx, idx)
        }
    }

    /// Apply a function to each component in a pre-order traversal.
    pub fn apply_pre_order<F>(self, mut f: F)
    where
        F: FnMut(&Context, CompIdx),
    {
        for &idx in self.order.iter().rev() {
            log::trace!("Pre-order: {}", idx);
            f(&self.ctx, idx)
        }
    }

    /// Take the [Context] from the post order structure.
    pub fn take(self) -> Context {
        self.ctx
    }

    fn process_cmd(
        ctx: &Context,
        comp: CompIdx,
        cmd: &Command,
        ts: &mut TopologicalSort<CompIdx>,
    ) {
        match cmd {
            Command::Instance(inst) => {
                let inst = ctx.comps.get(comp).instances().get(*inst);
                // If the instance is not an external, add a dependency edge
                if !ctx.comps.get(inst.comp).is_ext {
                    ts.add_dependency(comp, inst.comp);
                }
            }
            Command::ForLoop(fl) => {
                for cmd in &fl.body {
                    Traversal::process_cmd(ctx, comp, cmd, ts);
                }
            }
            Command::If(i) => {
                for cmd in &i.then {
                    Traversal::process_cmd(ctx, comp, cmd, ts);
                }
                for cmd in &i.alt {
                    Traversal::process_cmd(ctx, comp, cmd, ts);
                }
            }
            Command::Connect(_) | Command::Invoke(_) | Command::Fact(_) => (),
        }
    }
}

#[derive(Copy)]
/// A reference to a foreign key and its owner.
/// On its own, a foreign key is not very useful. We need provide it with a context
/// that can resolve the owner which can then resolve the underlying type.
/// However, we do not provide a way to extract the underyling `T`.
pub struct Foreign<T, C> {
    /// A reference to the underlying value.
    key: Idx<T>,
    /// A reference to the owner of the foreign key.
    owner: Idx<C>,
}

impl<T, C> Foreign<T, C> {
    pub fn new(key: Idx<T>, owner: Idx<C>) -> Self {
        Self { key, owner }
    }

    /// Map over the foreign key using the given context.
    /// We require a context to resolve the owner of the foreign key.
    pub fn map<X, F>(&self, ctx: &impl Ctx<C>, mut f: F) -> Foreign<X, C>
    where
        F: FnMut(&C, Idx<T>) -> Idx<X>,
    {
        let c_resolved = ctx.get(self.owner);
        Foreign {
            key: f(c_resolved, self.key),
            owner: self.owner,
        }
    }

    /// Runs a function used to unwrap the foreign type into a different type.
    pub fn unwrap<F, X>(&self, ctx: &impl Ctx<C>, mut f: F) -> X
    where
        F: FnMut(&C, Idx<T>) -> X,
    {
        let c_resolved = ctx.get(self.owner);
        f(c_resolved, self.key)
    }
}

impl<T, C> Clone for Foreign<T, C> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            owner: self.owner,
        }
    }
}

impl<T, C> PartialEq for Foreign<T, C> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.owner == other.owner
    }
}

impl<T, C> Eq for Foreign<T, C> {}

impl<T, C> std::hash::Hash for Foreign<T, C> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.owner.hash(state);
    }
}
