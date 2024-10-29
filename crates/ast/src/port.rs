use super::{Binding, Bundle, Expr, Id, Loc, Range, Time};
use fil_utils as utils;

/// A port definition in a [super::Signature].
#[derive(Clone)]
pub enum PortDef {
    Port {
        name: Loc<Id>,
        /// Liveness condition for the Port
        liveness: Loc<Range>,
        /// Bitwidth of the port
        bitwidth: Loc<Expr>,
    },
    Bundle(Bundle),
}
impl From<Bundle> for PortDef {
    fn from(b: Bundle) -> Self {
        Self::Bundle(b)
    }
}

impl PortDef {
    pub fn port(
        name: Loc<Id>,
        liveness: Loc<Range>,
        bitwidth: Loc<Expr>,
    ) -> Self {
        Self::Port {
            name,
            liveness,
            bitwidth,
        }
    }

    pub fn bundle(bundle: Bundle) -> Self {
        Self::Bundle(bundle)
    }

    /// Name of this Port
    pub fn name(&self) -> &Loc<Id> {
        match &self {
            PortDef::Port { name, .. } => name,
            PortDef::Bundle(b) => &b.name,
        }
    }
}
impl PortDef {
    /// Resolves all time expressions in this port definition
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        match self {
            PortDef::Port {
                name,
                liveness,
                bitwidth,
            } => PortDef::Port {
                name,
                liveness: liveness.map(|l| l.resolve_event(bindings)),
                bitwidth,
            },
            PortDef::Bundle(b) => {
                let t = b.typ.resolve_event(bindings);
                let bun = Bundle { typ: t, ..b };
                PortDef::Bundle(bun)
            }
        }
    }

    /// Resolves all width expressions in this port definition.
    /// Specifically:
    /// - The bitwidth of the port
    /// - The liveness condition
    pub fn resolve_exprs(self, bindings: &Binding<Expr>) -> Self {
        match self {
            PortDef::Port {
                name,
                liveness,
                bitwidth,
            } => PortDef::Port {
                name,
                liveness: liveness.map(|l| l.resolve_exprs(bindings)),
                bitwidth: bitwidth.map(|b| b.resolve(bindings)),
            },
            PortDef::Bundle(b) => PortDef::Bundle(b.resolve_exprs(bindings)),
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
impl InterfaceDef {
    pub fn new(name: Loc<Id>, event: Id) -> Self {
        Self { name, event }
    }
}
