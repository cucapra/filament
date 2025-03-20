use fil_ast as ast;
use fil_ir::{
    self as ir, AddCtx, Ctx, DenseIndexInfo, DisplayCtx, Idx, IndexStore,
    InterfaceSrc, MutCtx,
};
use fil_utils as utils;

use super::{Base, IntoBase, IntoUdl, Underlying};

#[derive(Clone)]
pub struct UnderlyingComp<'a>(&'a ir::Component);

impl<'a> UnderlyingComp<'a> {
    pub fn new(comp: &'a ir::Component) -> Self {
        Self(comp)
    }
    // XXX(rachitnigam): We should probably define a macro to dispatch these methods.
    pub fn cmds(&self) -> &Vec<ir::Command> {
        &self.0.cmds
    }
    pub fn is_ext(&self) -> bool {
        self.0.is_ext()
    }
    pub fn events(&self) -> &IndexStore<ir::Event> {
        self.0.events()
    }
    pub fn ports(&self) -> &IndexStore<ir::Port> {
        self.0.ports()
    }
    pub fn port_attrs(&self) -> &DenseIndexInfo<ir::Port, utils::PortAttrs> {
        &self.0.port_attrs
    }
    pub fn src_info(&self) -> &Option<InterfaceSrc> {
        &self.0.src_info
    }
    pub fn unannotated_ports(&self) -> &Vec<(ast::Id, u64)> {
        &self.0.unannotated_ports
    }
    pub fn exist_params(&self) -> impl Iterator<Item = ir::ParamIdx> + '_ {
        self.0.exist_params()
    }
    pub fn all_exist_assumes(&self) -> Vec<(ir::PropIdx, utils::GPosIdx)> {
        self.0.all_exist_assumes()
    }
    pub fn relevant_vars(
        &self,
        prop: Underlying<ir::Prop>,
    ) -> (Vec<Underlying<ir::Param>>, Vec<Underlying<ir::Event>>) {
        let (params, events) = prop.idx().relevant_vars(self.0);

        (
            params.into_iter().map(|p| p.ul()).collect(),
            events.into_iter().map(|e| e.ul()).collect(),
        )
    }
}

// The underlying component is a context for everything that a component is a context for.
impl<T> Ctx<T, Underlying<T>> for UnderlyingComp<'_>
where
    ir::Component: Ctx<T>,
{
    fn get(&self, k: Underlying<T>) -> &T {
        self.0.get(k.idx())
    }
}

impl<T> DisplayCtx<Underlying<T>> for UnderlyingComp<'_>
where
    ir::Component: DisplayCtx<Idx<T>>,
{
    fn write(
        &self,
        val: Underlying<T>,
        f: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
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

    pub fn push_port_attrs(
        &mut self,
        key: Base<ir::Port>,
        val: utils::PortAttrs,
    ) {
        self.0.port_attrs.push(key.get(), val);
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

    fn valid(&self, idx: Base<T>) -> bool {
        self.0.valid(idx.get())
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
        f: &mut impl std::io::Write,
    ) -> std::io::Result<()> {
        self.0.write(val.get(), f)
    }
}
