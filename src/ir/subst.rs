use super::{Component, Ctx, EventIdx, Expr, ExprIdx, ParamIdx, Time, TimeIdx};

pub struct Bind<K: Eq, V>(Vec<(K, V)>);

impl<K: Eq, V> Bind<K, V> {
    pub fn new(bind: Vec<(K, V)>) -> Self {
        Self(bind)
    }
}

impl<K, V> Bind<K, V>
where
    K: Eq,
{
    /// Get the binding associated with a particular key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.0.push((key, value));
    }
}

/// A substitution for a type `T` that contains type `K` inside it.
/// Substitutions cannot be applied with a type that implements [Ctx].
pub struct Subst<'a, T, K, V>
where
    K: Eq,
{
    base: T,
    bind: &'a Bind<K, V>,
}

impl<'a, T, K, V> Subst<'a, T, K, V>
where
    K: Eq,
{
    pub fn new(base: T, bind: &'a Bind<K, V>) -> Self {
        Self { base, bind }
    }

    /// Construct a new substitution with a different base
    pub fn new_base<U>(&self, new_base: U) -> Subst<'a, U, K, V> {
        Subst {
            base: new_base,
            bind: self.bind,
        }
    }

    /// Apply a function to the base and return a new substitution.
    ///
    /// This function returns a new substitution because the function performed on the
    /// base may return results that themselves need to be substituted.
    pub fn map<F, U>(self, mut f: F) -> Subst<'a, U, K, V>
    where
        F: FnMut(T) -> U,
    {
        Subst {
            base: f(self.base),
            bind: self.bind,
        }
    }

    /// Extract the underlying base with the substitution applied.
    ///
    /// Use this only if you don't care about unsubstituted variables showing up in the
    /// result. Otherwise, use [Subst::apply].
    pub fn take_without_apply(self) -> T {
        self.base
    }
}

impl<'a, T, K, V: Clone> Subst<'a, T, K, V>
where
    T: Foldable<K, V>,
    K: Eq,
{
    /// Evaluate this subsitution if the underlying type knows how to apply it.
    pub fn apply(&self, ctx: &mut T::Context) -> T {
        self.base
            .fold_with(ctx, &mut |k| self.bind.get(&k).cloned())
    }
}

/// A type that can be folded using a function that transforms all instances of
/// `K` using a `K` -> `V` binding for the implementing type.
///
/// As an example, the impl:
/// ```
/// impl Foldable<ParamIdx, ExprIdx> for TimeIdx { ... }
/// ```
/// Allows all uses of [ParamIdx] in [TimeIdx] to be resolved using a [ParamIdx]
/// to [ExprIdx] map.
pub trait Foldable<K, V>
where
    K: Eq,
    Self: Sized,
{
    type Context;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(K) -> Option<V>;
}

impl Foldable<ParamIdx, ExprIdx> for ExprIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(ParamIdx) -> Option<ExprIdx>,
    {
        match ctx.get(*self).clone() {
            Expr::Param(p) => subst_fn(p).unwrap_or(*self),
            Expr::Concrete(_) => *self,
            Expr::Bin { op, lhs, rhs } => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Expr::Bin { op, lhs, rhs })
            }
            Expr::Fn { op, args } => {
                let args = args
                    .iter()
                    .map(|arg| arg.fold_with(ctx, subst_fn))
                    .collect();
                ctx.add(Expr::Fn { op, args })
            }
        }
    }
}

impl Foldable<EventIdx, TimeIdx> for TimeIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(EventIdx) -> Option<TimeIdx>,
    {
        let &Time { event, offset } = ctx.get(*self);
        if let Some(time) = subst_fn(event) {
            // If we get a new time event, we need to take the old time's offset and add it to this.
            let new_time = ctx.get(time).clone();
            let new_offset = new_time.offset.add(offset, ctx);
            ctx.add(Time {
                event: new_time.event,
                offset: new_offset,
            })
        } else {
            *self
        }
    }
}

impl Foldable<ParamIdx, ExprIdx> for TimeIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(ParamIdx) -> Option<ExprIdx>,
    {
        let &Time { event, offset } = ctx.get(*self);
        let new_offset = offset.fold_with(ctx, subst_fn);
        if new_offset == offset {
            return *self;
        }
        ctx.add(Time {
            event,
            offset: new_offset,
        })
    }
}
