use super::{Binding, PortParam};

/// Representation of a width in the program
pub trait WidthRep
where
    Self: Sized + Clone + Eq + Ord + std::fmt::Display + std::fmt::Debug,
{
    /// Construct a concrete width
    fn concerete(n: u64) -> Self;
    /// Resolve the width given a binding
    fn resolve<W: WidthRep>(&self, bindings: &Binding<W>) -> Option<W>;
}

impl WidthRep for u64 {
    fn concerete(n: u64) -> Self {
        n
    }
    fn resolve<W: WidthRep>(&self, _: &Binding<W>) -> Option<W> {
        Some(W::concerete(*self))
    }
}
impl WidthRep for PortParam {
    fn concerete(n: u64) -> Self {
        PortParam::Const(n)
    }
    fn resolve<W: WidthRep>(&self, bindings: &Binding<W>) -> Option<W> {
        match self {
            PortParam::Const(c) => Some(W::concerete(*c)),
            PortParam::Var(v) => bindings.find(v).cloned(),
        }
    }
}
