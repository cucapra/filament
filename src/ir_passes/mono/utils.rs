use crate::{
    ast,
    ir::{self, Ctx, IndexStore, InterfaceSrc, Interned, MutCtx},
    utils::{self, Idx},
};

pub struct UnderlyingComp<'a>(&'a ir::Component);

impl<'a> UnderlyingComp<'a> {
    pub fn new(comp: &'a ir::Component) -> Self {
        Self(comp)
    }
    pub fn comp(&self) -> &ir::Component {
        self.0
    }
    pub fn cmds(&self) -> &Vec<ir::Command> {
        &self.0.cmds
    }
    pub fn is_ext(&self) -> bool {
        self.0.is_ext
    }
    pub fn events(&self) -> &IndexStore<ir::Event> {
        self.0.events()
    }
    pub fn exprs(&self) -> &Interned<ir::Expr> {
        self.0.exprs()
    }
    pub fn params(&self) -> &IndexStore<ir::Param> {
        self.0.params()
    }
    pub fn ports(&self) -> &IndexStore<ir::Port> {
        self.0.ports()
    }
    pub fn src_info(&self) -> &Option<InterfaceSrc> {
        &self.0.src_info
    }
    pub fn unannotated_ports(&self) -> &Vec<(ast::Id, u64)> {
        &self.0.unannotated_ports
    }
}

pub trait UnderlyingCtx<T> {
    fn get(&self, k: Underlying<T>) -> &T;
}

impl<'a, T> UnderlyingCtx<T> for UnderlyingComp<'a>
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Underlying<T>) -> &T {
        self.0.get(k.idx())
    }
}

pub struct BaseComp(ir::Component);

impl BaseComp {
    pub fn new(comp: ir::Component) -> Self {
        Self(comp)
    }

    pub fn comp(&self) -> &ir::Component {
        &self.0
    }

    pub fn set_unannotated_ports(&mut self, other: Vec<(ast::Id, u64)>) {
        self.0.unannotated_ports = Box::new(other);
    }
    pub fn set_src_info(&mut self, other: Option<InterfaceSrc>) {
        self.0.src_info = other;
    }

    pub fn extend_one(&mut self, other: Option<ir::Command>) {
        self.0.cmds.extend(other);
    }
    pub fn extend_cmds(&mut self, other: Vec<ir::Command>) {
        self.0.cmds.extend(other);
    }
    pub fn num(&mut self, n: u64) -> Base<ir::Expr> {
        Base::new(self.0.add(ir::Expr::Concrete(n)))
    }
    pub fn bin(&mut self, expr: ir::Expr) -> Base<ir::Expr> {
        Base::new(self.0.bin(expr))
    }
    pub fn func(&mut self, expr: ir::Expr) -> Base<ir::Expr> {
        Base::new(self.0.func(expr))
    }
    pub fn resolve_prop(&mut self, prop: ir::Prop) -> Base<ir::Prop> {
        Base::new(self.0.resolve_prop(prop))
    }
}

pub trait BaseCtx<T> {
    fn get(&self, k: Base<T>) -> &T;
    fn add(&mut self, val: T) -> Base<T>;
}

pub trait MutBaseCtx<T> {
    fn get_mut(&mut self, k: Base<T>) -> &mut T;
}

impl<T> MutBaseCtx<T> for BaseComp
where
    ir::Component: MutCtx<T>,
{
    fn get_mut(&mut self, k: Base<T>) -> &mut T {
        self.0.get_mut(k.get())
    }
}

impl<T> BaseCtx<T> for BaseComp
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Base<T>) -> &T {
        self.0.get(k.get())
    }

    fn add(&mut self, val: T) -> Base<T> {
        Base::new(self.0.add(val))
    }
}

/// Wraps an Idx that is meaningful in the base component, which are the new components
/// that we build during monomorphization. As we visit parts of the underlying (pre-mono)
/// component, we need to build new monomorphized structures and add them to the new component.
/// In many cases, we keep these new Idxs around (to keep track of what's been monomorphized
/// already, as well as to maintain information between ports/invokes, for example). The point
/// of this wrapper is to make it clear that this Idx is meaningful only in the new component
/// we are constructing.
pub struct Base<T> {
    idx: Idx<T>,
}

impl<T> Base<T> {
    pub fn new(idx: Idx<T>) -> Self {
        Self { idx }
    }

    pub fn get(&self) -> Idx<T> {
        self.idx
    }
}

impl<T> Eq for Base<T> {}
impl<T> PartialEq for Base<T> {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}
impl<T> std::hash::Hash for Base<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}
impl<T> Clone for Base<T> {
    fn clone(&self) -> Self {
        Self { idx: self.idx }
    }
}
impl<T> PartialOrd for Base<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx.partial_cmp(&other.idx)
    }
}
impl<T> Ord for Base<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}
impl<T> Default for Base<T> {
    fn default() -> Self {
        Self { idx: Idx::UNKNOWN }
    }
}

impl<T> Copy for Base<T> {}
impl<T> utils::IdxLike<T> for Base<T> {
    const UNKNOWN: Self = Self { idx: Idx::UNKNOWN };

    fn new(idx: usize) -> Self {
        Self { idx: Idx::new(idx) }
    }

    fn get(self) -> usize {
        self.idx.get()
    }
}

/// Wraps an Idx that is meaningful in the underlying component, which are the existing pre-monomorphization
/// components. These Idxs get passed around between a lot of functions and mappings during monomorphization,
/// so it becomes hard to keep track of which Idx belongs where. This wrapper makes it distinct from base Idxs.
pub struct Underlying<T> {
    idx: Idx<T>,
}

impl<T> Underlying<T> {
    pub fn new(idx: Idx<T>) -> Self {
        Self { idx }
    }

    pub fn idx(&self) -> Idx<T> {
        self.idx
    }
}

impl<T> Eq for Underlying<T> {}
impl<T> PartialEq for Underlying<T> {
    fn eq(&self, other: &Self) -> bool {
        self.idx() == other.idx()
    }
}
impl<T> std::hash::Hash for Underlying<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.idx.hash(state)
    }
}

impl<T> Clone for Underlying<T> {
    fn clone(&self) -> Self {
        Self { idx: self.idx }
    }
}
impl<T> Copy for Underlying<T> {}
impl<T> PartialOrd for Underlying<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.idx.partial_cmp(&other.idx)
    }
}
impl<T> Ord for Underlying<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.idx.cmp(&other.idx)
    }
}

impl<T> utils::IdxLike<T> for Underlying<T> {
    const UNKNOWN: Self = Self { idx: Idx::UNKNOWN };

    fn new(idx: usize) -> Self {
        Self { idx: Idx::new(idx) }
    }

    fn get(self) -> usize {
        self.idx.get()
    }
}
