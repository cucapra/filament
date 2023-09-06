use fil_ast as ast;
use fil_ir::{
    self as ir, AddCtx, Ctx, DisplayCtx, Idx, IndexStore, InterfaceSrc,
    Interned, MutCtx,
};

use super::{Base, IntoBase, Underlying};

#[derive(Clone)]
pub struct UnderlyingComp<'a>(&'a ir::Component);

impl<'a> UnderlyingComp<'a> {
    pub fn new(comp: &'a ir::Component) -> Self {
        Self(comp)
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

// The underlying component is a context for everything that a component is a context for.
impl<'a, T> Ctx<T, Underlying<T>> for UnderlyingComp<'a>
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Underlying<T>) -> &T {
        self.0.get(k.idx())
    }
}

impl<'a, T> DisplayCtx<Underlying<T>> for UnderlyingComp<'a>
where
    ir::Component: DisplayCtx<Idx<T>>,
{
    fn write(
        &self,
        val: Underlying<T>,
        f: &mut impl std::fmt::Write,
    ) -> std::fmt::Result {
        self.0.write(val.idx(), f)
    }
}

pub struct BaseComp(ir::Component);

impl BaseComp {
    pub fn take(self) -> ir::Component {
        self.0
    }

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

    pub fn extend_cmds(
        &mut self,
        other: impl IntoIterator<Item = ir::Command>,
    ) {
        self.0.cmds.extend(other);
    }

    pub fn num(&mut self, n: u64) -> Base<ir::Expr> {
        self.0.add(ir::Expr::Concrete(n)).base()
    }
    pub fn bin(&mut self, expr: ir::Expr) -> Base<ir::Expr> {
        self.0.bin(expr).base()
    }
    pub fn func(&mut self, expr: ir::Expr) -> Base<ir::Expr> {
        self.0.func(expr).base()
    }
    pub fn resolve_prop(&mut self, prop: ir::Prop) -> Base<ir::Prop> {
        self.0.resolve_prop(prop).base()
    }
}

impl<T> MutCtx<T, Base<T>> for BaseComp
where
    ir::Component: MutCtx<T>,
{
    fn get_mut(&mut self, k: Base<T>) -> &mut T {
        self.0.get_mut(k.get())
    }

    fn delete(&mut self, k: Base<T>) {
        self.0.delete(k.get());
    }
}

impl<T> Ctx<T, Base<T>> for BaseComp
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Base<T>) -> &T {
        self.0.get(k.get())
    }
}

impl<T> AddCtx<T, Base<T>> for BaseComp
where
    ir::Component: AddCtx<T>,
{
    fn add(&mut self, val: T) -> Base<T> {
        self.0.add(val).base()
    }
}

impl<T> DisplayCtx<Base<T>> for BaseComp
where
    ir::Component: DisplayCtx<Idx<T>>,
{
    fn write(
        &self,
        val: Base<T>,
        f: &mut impl std::fmt::Write,
    ) -> std::fmt::Result {
        self.0.write(val.get(), f)
    }
}
