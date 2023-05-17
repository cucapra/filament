// All defines (ports, params, events) need to a part of the component
// definition while expressions like constraints can be a part of the signature

use crate::{ast, idx, utils::Idx};
use std::collections::HashMap;

pub type ExprIdx = idx!(Expr);
pub type ParamIdx = idx!(Param);
pub type PortIdx = idx!(Port);
pub type EventIdx = idx!(Event);
pub type TimeIdx = idx!(Time);
pub type RangeIdx = idx!(Range);

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Expr {
    Param(ParamIdx),
    Concrete(u64),
    Bin {
        op: ast::Op,
        lhs: ExprIdx,
        rhs: ExprIdx,
    },
    Fn {
        op: ast::UnFn,
        args: Box<[ExprIdx]>,
    },
}

// impl ExprIdx {
//     /// Resolve the expression given a binding from parameters to expressions
//     pub fn resolve<'a, C>(
//         self,
//         ctx: &mut C,
//         subst: &'a Subst<'a, Self, ParamIdx, Self>,
//     ) -> Self
//     where
//         C: Ctx<Expr>,
//     {
//         match ctx.get(self) {
//             Expr::Param(p) => subst.bind.get(p).copied().unwrap_or(self),
//             Expr::Concrete(_) => self,
//             Expr::Bin { op, lhs, rhs } => {
//                 let lhs = lhs.resolve(ctx, subst);
//                 let rhs = rhs.resolve(ctx, subst);
//                 ctx.add(Expr::Bin { op: *op, lhs, rhs })
//             }
//             Expr::Fn { op, args } => {
//                 let args =
//                     args.iter().map(|&a| a.resolve(ctx, subst)).collect();
//                 ctx.add(Expr::Fn {
//                     op: op.clone(),
//                     args,
//                 })
//             }
//         }
//     }
// }

impl std::ops::Add for ExprIdx {
    type Output = Expr;

    fn add(self, rhs: Self) -> Self::Output {
        Expr::Bin {
            op: ast::Op::Add,
            lhs: self,
            rhs,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// A temporal event. Represents an offset from the start of the event.
pub struct Time {
    event: EventIdx,
    offset: ExprIdx,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// An interval of time
pub struct Range {
    start: TimeIdx,
    end: TimeIdx,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// A port tracks its definition and liveness.
/// A port in the IR generalizes both bundles and normal ports.
pub struct Port {
    len: ExprIdx,
    width: ExprIdx,
    liveness: RangeIdx,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// Parameters with an optional initial value
pub struct Param {
    default: ExprIdx,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// Events must have a delay and an optional default value
pub struct Event {
    delay: ExprIdx,
    default: Option<ExprIdx>,
}

/// An indexed storage for an interned type. Keeps a HashMap to provide faster reverse mapping
/// from the value to the index.
/// Useful for types that are added continuously throughout the compiler's execution.
///
/// ## Safety
/// The data structure internally stores a pointer to each value stored. This is safe
/// because we do not allow deletion of keys.
/// If a key is ever deleted, a call to `get` will return a dangling pointer.
struct Indexed<T>
where
    T: Eq + std::hash::Hash,
{
    store: Vec<*const T>,
    map: HashMap<T, Idx<T>>,
}

impl<T> Indexed<T>
where
    T: Eq + std::hash::Hash,
{
    fn add(&mut self, val: T) -> Idx<T> {
        // If the value is already in the map, return the index
        if let Some(idx) = self.map.get(&val) {
            return *idx;
        }
        // Otherwise, add the value to the store and map
        let idx = Idx::new(self.store.len());
        self.store.push(&val);
        self.map.insert(val, idx);
        idx
    }

    fn get(&self, idx: Idx<T>) -> &T {
        let pointer = self.store[idx.get()];
        unsafe { pointer.as_ref().unwrap() }
    }
}

/// A small indexed storage for type. Prefer this over [Indexed] when we
/// don't expect the number of elements to be large.
/// Adding new elements is an O(n) operation since we search the store for the
/// element first.
struct SmallIndexed<T>
where
    T: Eq,
{
    store: Vec<T>,
}

impl<T> SmallIndexed<T>
where
    T: Eq,
{
    fn add(&mut self, val: T) -> Idx<T> {
        // If the value is already in the map, return the index
        if let Some(idx) = self.store.iter().position(|v| *v == val) {
            return Idx::new(idx);
        }
        // Otherwise, add the value to the store and map
        let idx = Idx::new(self.store.len());
        self.store.push(val);
        idx
    }

    fn get(&self, idx: Idx<T>) -> &T {
        &self.store[idx.get()]
    }
}

pub struct Component {
    // Component defined values. Once created, we don't expect too many of these
    // to be created.
    /// Ports defined by the component
    ports: SmallIndexed<Port>,
    /// Parameters defined the component
    params: SmallIndexed<Param>,
    /// Events defined by the component
    events: SmallIndexed<Event>,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    exprs: Indexed<Expr>,
    /// Interned times
    times: Indexed<Time>,
    /// Interned ranges
    ranges: Indexed<Range>,
}

/// A context for interning values.
/// In addition to adding and getting values, the context also supports applying
/// a substitution to a value.
trait Ctx<T> {
    /// Add a value to the context
    fn add(&mut self, val: T) -> Idx<T>;
    /// Get the information associated with a value
    fn get(&self, idx: Idx<T>) -> &T;
}

impl Ctx<Expr> for Component {
    fn add(&mut self, val: Expr) -> ExprIdx {
        self.exprs.add(val)
    }

    fn get(&self, idx: ExprIdx) -> &Expr {
        self.exprs.get(idx)
    }
}

impl Ctx<Time> for Component {
    fn add(&mut self, val: Time) -> TimeIdx {
        self.times.add(val)
    }

    fn get(&self, idx: TimeIdx) -> &Time {
        self.times.get(idx)
    }
}

struct Bind<'a, K, V>(&'a [(K, V)])
where
    K: Eq;

impl<K: Eq, V> Clone for Bind<'_, K, V> {
    fn clone(&self) -> Self {
        Bind(self.0)
    }
}

impl<K, V> Bind<'_, K, V>
where
    K: Eq,
{
    /// Get the binding associated with a particular key
    pub fn get(&self, key: &K) -> Option<&V> {
        self.0
            .iter()
            .find_map(|(k, v)| if k == key { Some(v) } else { None })
    }
}

/// A substitution for a type `T` that contains type `K` inside it.
/// Substitutions cannot be applied with a type that implements [Ctx].
struct Subst<'a, T, K, V>
where
    K: Eq,
{
    base: T,
    bind: Bind<'a, K, V>,
}

impl<'a, T, K, V> Subst<'a, T, K, V>
where
    K: Eq,
{
    pub fn new(base: T, bind: Bind<'a, K, V>) -> Self {
        Self { base, bind }
    }

    /// Construct a new substitution with a different base
    pub fn new_base<U>(&self, new_base: U) -> Subst<'a, U, K, V> {
        Subst {
            base: new_base,
            bind: self.bind.clone(),
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
            bind: self.bind.clone(),
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
    fn apply(&self, ctx: &mut T::Context) -> T {
        self.base
            .fold_with(ctx, &mut |k| self.bind.get(&k).cloned())
    }
}

/// A type that can be folded using a function that transforms all instances of
/// `K` to `V` using the given context.
trait Foldable<K, V>
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
            let new_offset = ctx.add(new_time.offset + offset);
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
