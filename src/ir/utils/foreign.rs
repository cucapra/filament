use crate::{
    ir::Ctx,
    utils::{Idx, IdxLike},
};

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
