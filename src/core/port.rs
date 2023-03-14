use super::{Expr, Id, Loc, Range, Time};
use crate::utils::Binding;
use std::fmt::Display;

/// A port definition in a [super::Signature].
#[derive(Clone)]
pub struct PortDef {
    /// Name of the port
    name: Loc<Id>,
    /// Liveness condition for the Port
    liveness: Loc<Range>,
    /// Bitwidth of the port
    bitwidth: Loc<Expr>,
}

impl PortDef {
    pub fn new(
        name: Loc<Id>,
        liveness: Loc<Range>,
        bitwidth: Loc<Expr>,
    ) -> Self {
        Self {
            name,
            liveness,
            bitwidth,
        }
    }

    /// Name of this Port
    pub fn name(&self) -> &Loc<Id> {
        &self.name
    }

    pub fn bitwidth(&self) -> &Loc<Expr> {
        &self.bitwidth
    }

    pub fn liveness(&self) -> &Loc<Range> {
        &self.liveness
    }
}
impl Display for PortDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", *self.liveness, self.name, *self.bitwidth,)
    }
}
impl PortDef {
    /// Resolves all time expressions in this port definition
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        Self {
            liveness: self.liveness.map(|l| l.resolve_event(bindings)),
            ..self
        }
    }

    /// Resolves all width expressions in this port definition.
    /// Specifically:
    /// - The bitwidth of the port
    /// - The liveness condition
    pub fn resolve_offset(self, bindings: &Binding<Expr>) -> Self {
        Self {
            bitwidth: self.bitwidth.map(|b| b.resolve(bindings)),
            liveness: self.liveness.map(|l| l.resolve_exprs(bindings)),
            ..self
        }
    }
}

#[derive(Clone)]
pub struct InterfaceDef {
    /// Name of the port
    pub name: Loc<Id>,
    /// Event that this port is an evidence of
    pub event: Id,
}
impl Display for InterfaceDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@interface[{}] {}: 1", self.event, self.name)
    }
}

impl InterfaceDef {
    pub fn new(name: Loc<Id>, event: Id) -> Self {
        Self { name, event }
    }
}

impl From<InterfaceDef> for PortDef {
    fn from(id: InterfaceDef) -> Self {
        let start = Time::from(id.event);
        let end = start.clone().increment(1.into());
        PortDef::new(
            id.name,
            Loc::unknown(Range::new(start, end)),
            Loc::unknown(Expr::from(1)),
        )
    }
}
