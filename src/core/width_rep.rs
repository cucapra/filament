use super::{Binding, Id, PortParam};

/// Representation of a width in the program
pub trait WidthRep
where
    Self: Sized + Clone + Eq + Ord + std::fmt::Display + std::fmt::Debug,
{
    /// Construct a concrete width
    fn concrete(n: u64) -> Self;
    /// Representation of parameter
    fn param(v: Id) -> Self;
    /// Resolve the width given a binding
    fn resolve<W: WidthRep>(&self, bindings: &Binding<W>) -> Option<W>;
    /// Lift the width to a port param
    fn lift(self) -> PortParam;
}

impl WidthRep for u64 {
    fn concrete(n: u64) -> Self {
        n
    }
    fn param(_: Id) -> Self {
        unreachable!("Cannot represent parameter as a concrete width")
    }
    fn resolve<W: WidthRep>(&self, _: &Binding<W>) -> Option<W> {
        Some(W::concrete(*self))
    }
    fn lift(self) -> PortParam {
        PortParam::Const(self)
    }
}
impl WidthRep for PortParam {
    fn concrete(n: u64) -> Self {
        PortParam::Const(n)
    }
    fn param(v: Id) -> Self {
        PortParam::Var(v)
    }
    fn resolve<W: WidthRep>(&self, bindings: &Binding<W>) -> Option<W> {
        match self {
            PortParam::Const(c) => Some(W::concrete(*c)),
            PortParam::Var(v) => bindings.find(v).cloned(),
        }
    }
    fn lift(self) -> PortParam {
        self
    }
}
