use super::{AddCtx, Command, CompIdx, Context};
use super::{Ctx, MutCtx};
use crate::utils::{self, Idx, IdxLike};
use bitvec::vec::BitVec;
use itertools::Itertools;
use std::{
    collections::HashMap, fmt::Display, iter::IntoIterator,
    marker::PhantomData, rc::Rc,
};
use topological_sort::TopologicalSort;

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

#[derive(Clone)]
/// An indexed store for a type.
/// Unlike [Interned], this data structure does not deduplicate values and supports mutation of values and removal of indices.
pub struct IndexStore<T, I = utils::Idx<T>>
where
    I: utils::IdxLike<T>,
{
    store: Vec<T>,
    /// Tracks which indices have been marked invalid.
    valid: BitVec,
    /// The kind of index used to access the store.
    _type: PhantomData<I>,
}

impl<T> Default for IndexStore<T> {
    fn default() -> Self {
        Self {
            store: Vec::new(),
            valid: BitVec::new(),
            _type: PhantomData,
        }
    }
}

impl<T, I> IndexStore<T, I>
where
    I: utils::IdxLike<T>,
{
    /// A a value to the store and return the index.
    pub fn add(&mut self, val: T) -> I {
        // Add the value to the store and return index
        let idx = I::new(self.store.len());
        self.store.push(val);
        self.valid.push(true);
        idx
    }

    /// Mark an index as invalid.
    /// The underlying data structure does not actually deallocate or reuse the value associated with the index.
    pub fn delete(&mut self, idx: I) {
        let i = idx.get();
        // Should not attempt to remove an index that has already been removed
        assert!(self.valid[i], "Attempted to delete invalid index {i}.");
        self.valid.set(i, false);
    }

    /// Returns true iff the index is valid.
    pub fn is_valid(&self, idx: I) -> bool {
        self.valid[idx.get()]
    }

    /// Get the value associated with the index.
    pub fn get(&self, idx: I) -> &T {
        let i = idx.get();
        assert!(self.valid[i], "Attempted to get invalid index {i}.");
        &self.store[i]
    }

    /// Get a mutable reference to the value associated with the index.
    pub fn get_mut(&mut self, idx: I) -> &mut T {
        let i = idx.get();
        assert!(self.valid[i], "Attempted to get invalid index {i}.");
        &mut self.store[i]
    }

    /// Number of elements in the store
    pub fn len(&self) -> usize {
        self.store.len()
    }

    /// Check if the store is empty
    pub fn is_empty(&self) -> bool {
        self.store.is_empty()
    }

    /// Iterate over the valid indices in the store.
    /// This can be useful because it allows mutable borrows of the owners of the store.
    pub fn idx_iter(&self) -> impl Iterator<Item = I> {
        self.valid
            .clone()
            .into_iter()
            .enumerate()
            .filter(|(_, valid)| *valid)
            .map(|(idx, _)| I::new(idx))
    }

    /// Iterate over the valid indices and the values in the store.
    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> + '_ {
        self.store
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.valid[*idx])
            .map(|(idx, val)| (I::new(idx), val))
    }

    /// Mutable iteration over the valid indices and the values in the store.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (I, &mut T)> {
        self.store
            .iter_mut()
            .enumerate()
            .filter(|(idx, _)| self.valid[*idx])
            .map(|(idx, val)| (I::new(idx), val))
    }

    pub(super) fn checked_add(&mut self, idx: I, val: T) {
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
///
/// Note: This data structure has no way to track which indices are valid so it
/// is up to the user to ensure that the indices are valid by calling [IndexStore::is_valid]
/// on a valid index.
pub struct DenseIndexInfo<T, V, K = utils::Idx<T>>
where
    K: utils::IdxLike<T>,
{
    store: Vec<V>,
    _key_typ: PhantomData<T>,
    _idx_typ: PhantomData<K>,
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

impl<T, V, I: utils::IdxLike<T>> std::ops::Index<I>
    for DenseIndexInfo<T, V, I>
{
    type Output = V;

    fn index(&self, idx: I) -> &Self::Output {
        self.get(idx)
    }
}

/// Defines a traversal of the components.
/// Equivalent to the ast traversal except works over IR components.
/// There is an edge between src -> dst if `src` instantiates an instance of `dst`
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
        assert!(
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
            Command::Connect(_)
            | Command::BundleDef(_)
            | Command::Invoke(_)
            | Command::Fact(_) => (),
        }
    }
}

/// A reference to a foreign key and its owner.
/// On its own, a foreign key is not very useful. We need provide it with a context
/// that can resolve the owner which can then resolve the underlying type.
/// However, we do not provide a way to extract the underyling `T`.
pub struct Foreign<T, C>
where
    C: Ctx<T>,
{
    /// A reference to the underlying value.
    key: Idx<T>,
    /// A reference to the owner of the foreign key.
    owner: Idx<C>,
}

impl<T, C> Foreign<T, C>
where
    C: Ctx<T>,
{
    pub fn key(&self) -> Idx<T> {
        self.key
    }

    pub fn owner(&self) -> Idx<C> {
        self.owner
    }

    pub fn new(key: Idx<T>, owner: Idx<C>) -> Self {
        Self { key, owner }
    }

    /// Map over the foreign key using the given context.
    /// We require a context to resolve the owner of the foreign key.
    pub fn map<X, F>(&self, mut f: F, ctx: &impl Ctx<C>) -> Foreign<X, C>
    where
        F: FnMut(Idx<T>, &C) -> Idx<X>,
        C: Ctx<X>,
    {
        let c_resolved = ctx.get(self.owner);
        Foreign {
            key: f(self.key, c_resolved),
            owner: self.owner,
        }
    }

    /// Runs a function used to unwrap the foreign type into a different type.
    /// Shouldn't be used if `X` is an `Idx<T>` as this index will be unsafe.
    pub fn apply<F, X>(&self, mut f: F, ctx: &impl Ctx<C>) -> X
    where
        F: FnMut(Idx<T>, &C) -> X,
    {
        let c_resolved = ctx.get(self.owner);
        f(self.key, c_resolved)
    }

    /// Returns the underlying key and owner.
    pub fn take(&self) -> (Idx<T>, Idx<C>) {
        (self.key, self.owner)
    }
}

impl<T, C> IdxLike<T> for Foreign<T, C>
where
    C: Ctx<T>,
{
    const UNKNOWN: Self = Foreign {
        key: Idx::UNKNOWN,
        owner: Idx::UNKNOWN,
    };

    fn new(idx: usize) -> Self {
        Self {
            key: Idx::new(idx),
            owner: Idx::UNKNOWN,
        }
    }

    fn get(self) -> usize {
        self.key.get()
    }
}

impl<T, C> Ord for Foreign<T, C>
where
    C: Ctx<T>,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.owner.cmp(&other.owner).then(self.key.cmp(&other.key))
    }
}

impl<T, C> Clone for Foreign<T, C>
where
    C: Ctx<T>,
{
    fn clone(&self) -> Self {
        *self
    }
}

/// All foreigns are [Copy]
impl<T, C> Copy for Foreign<T, C> where C: Ctx<T> {}

impl<T, C> PartialOrd for Foreign<T, C>
where
    C: Ctx<T>,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T, C> PartialEq for Foreign<T, C>
where
    C: Ctx<T>,
{
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.owner == other.owner
    }
}

impl<T, C> Eq for Foreign<T, C> where C: Ctx<T> {}

impl<T, C> std::hash::Hash for Foreign<T, C>
where
    C: Ctx<T>,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.owner.hash(state);
    }
}

impl<T> Ctx<T> for IndexStore<T> {
    fn get(&self, idx: Idx<T>) -> &T {
        self.get(idx)
    }
}

impl<T> AddCtx<T> for IndexStore<T> {
    fn add(&mut self, val: T) -> Idx<T> {
        self.add(val)
    }
}

impl<T> MutCtx<T> for IndexStore<T> {
    fn get_mut(&mut self, idx: Idx<T>) -> &mut T {
        self.get_mut(idx)
    }

    fn delete(&mut self, idx: Idx<T>) {
        self.delete(idx)
    }
}

#[macro_export]
/// Creates a constructor function for a binary operator.
/// Example: ```
/// construct_binop!(
/// <impl Ctx<Expr>>(ExprIdx::bin, Expr) => Expr;
///     add = ast::Op::Add;
///     mul = ast::Op::Mul;
///     div = ast::Op::Div;
///     rem = ast::Op::Mod;
/// );
/// ```
macro_rules! construct_binop {
    (<$ctx: ty> ($constructor: expr, $in: ty) => $out: ty;
    $($name:ident = $op: expr);* ;) => {
        impl $in {
            $(pub fn $name(self, other: $in, ctx: &mut $ctx) -> $crate::utils::Idx<$out> {
                ctx.add($constructor(
                    self, other, $op
                ))
            })*
        }
    };
}
