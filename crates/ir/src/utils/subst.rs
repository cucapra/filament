use crate::{
    AddCtx, CmpOp, Component, Ctx, EventIdx, Expr, ExprIdx, ParamIdx, Prop,
    PropIdx, Time, TimeIdx,
};

/// A binding from K to V.
pub struct Bind<K: Eq, V>(Vec<(K, V)>);

impl<K: Eq, V> Bind<K, V> {
    pub fn new(bind: impl IntoIterator<Item = (K, V)>) -> Self {
        Self(bind.into_iter().collect())
    }
}

impl<K: Eq + std::fmt::Debug, V: std::fmt::Debug> std::fmt::Debug
    for Bind<K, V>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (k, v) in self.0.iter() {
            write!(f, "{:?} -> {:?}, ", k, v)?;
        }
        Ok(())
    }
}

impl<K, V> Bind<K, V>
where
    K: Eq,
{
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get the binding associated with a particular key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0.iter().find_map(|(k, v)| (k == key).then_some(v))
    }

    /// Insert a new binding
    pub fn push(&mut self, key: K, value: V) {
        self.0.push((key, value));
    }

    /// Extend the binding
    pub fn extend(&mut self, other: impl Iterator<Item = (K, V)>) {
        self.0.extend(other);
    }

    /// Remove the last binding and return it
    pub fn pop(&mut self) -> Option<(K, V)> {
        self.0.pop()
    }

    /// Remove the last n bindings
    /// Panics if `n` is greater than the number of bindings
    pub fn pop_n(&mut self, n: usize) {
        let len = self.0.len();
        assert!(n <= len);
        self.0.truncate(len - n);
    }

    /// Iterate over the binding
    pub fn iter(&self) -> impl Iterator<Item = &(K, V)> {
        self.0.iter()
    }
}

impl<K, V> Bind<K, V>
where
    K: Eq,
    K: Clone,
    V: Clone,
{
    pub fn inner(&self) -> Vec<(K, V)> {
        let Bind(v) = self;
        v.to_vec()
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

impl<T, K, V: Clone> Subst<'_, T, K, V>
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
            Expr::If { cond, then, alt } => {
                let cond = cond.fold_with(ctx, subst_fn);
                let then = then.fold_with(ctx, subst_fn);
                let alt = alt.fold_with(ctx, subst_fn);
                ctx.add(Expr::If { cond, then, alt })
            }
        }
    }
}

impl Foldable<ParamIdx, ExprIdx> for PropIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(ParamIdx) -> Option<ExprIdx>,
    {
        match ctx.get(*self).clone() {
            Prop::True | Prop::False => *self,
            Prop::Cmp(CmpOp { op, lhs, rhs }) => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::Cmp(CmpOp { op, lhs, rhs }))
            }
            Prop::TimeCmp(CmpOp { op, lhs, rhs }) => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::TimeCmp(CmpOp { op, lhs, rhs }))
            }
            Prop::TimeSubCmp(CmpOp { op, lhs, rhs }) => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::TimeSubCmp(CmpOp { op, lhs, rhs }))
            }
            Prop::Not(p) => {
                let p = p.fold_with(ctx, subst_fn);
                p.not(ctx)
            }
            Prop::And(l, r) => {
                let l = l.fold_with(ctx, subst_fn);
                let r = r.fold_with(ctx, subst_fn);
                l.and(r, ctx)
            }
            Prop::Or(l, r) => {
                let l = l.fold_with(ctx, subst_fn);
                let r = r.fold_with(ctx, subst_fn);
                l.or(r, ctx)
            }
            Prop::Implies(a, c) => {
                let a = a.fold_with(ctx, subst_fn);
                let c = c.fold_with(ctx, subst_fn);
                a.implies(c, ctx)
            }
        }
    }
}

impl Foldable<EventIdx, TimeIdx> for ExprIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(EventIdx) -> Option<TimeIdx>,
    {
        match ctx.get(*self).clone() {
            Expr::Param(_) | Expr::Concrete(_) => *self,
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
            Expr::If { cond, then, alt } => {
                let cond = cond.fold_with(ctx, subst_fn);
                let then = then.fold_with(ctx, subst_fn);
                let alt = alt.fold_with(ctx, subst_fn);
                ctx.add(Expr::If { cond, then, alt })
            }
        }
    }
}

impl Foldable<EventIdx, TimeIdx> for PropIdx {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(EventIdx) -> Option<TimeIdx>,
    {
        match ctx.get(*self).clone() {
            Prop::True | Prop::False => *self,
            Prop::Cmp(CmpOp { op, lhs, rhs }) => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::Cmp(CmpOp { op, lhs, rhs }))
            }
            Prop::TimeCmp(CmpOp { op, lhs, rhs }) => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::TimeCmp(CmpOp { op, lhs, rhs }))
            }
            Prop::TimeSubCmp(CmpOp { op, lhs, rhs }) => {
                let lhs = lhs.fold_with(ctx, subst_fn);
                let rhs = rhs.fold_with(ctx, subst_fn);
                ctx.add(Prop::TimeSubCmp(CmpOp { op, lhs, rhs }))
            }
            Prop::Not(p) => {
                let p = p.fold_with(ctx, subst_fn);
                p.not(ctx)
            }
            Prop::And(l, r) => {
                let l = l.fold_with(ctx, subst_fn);
                let r = r.fold_with(ctx, subst_fn);
                l.and(r, ctx)
            }
            Prop::Or(l, r) => {
                let l = l.fold_with(ctx, subst_fn);
                let r = r.fold_with(ctx, subst_fn);
                l.or(r, ctx)
            }
            Prop::Implies(a, c) => {
                let a = a.fold_with(ctx, subst_fn);
                let c = c.fold_with(ctx, subst_fn);
                a.implies(c, ctx)
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
