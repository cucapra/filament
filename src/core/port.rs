use super::{Expr, Id, Range, Time};
use crate::utils::Binding;
use crate::{errors::WithPos, utils::GPosIdx};
use std::fmt::Display;

#[derive(Clone)]
pub struct PortDef {
    /// Name of the port
    pub name: Id,
    /// Liveness condition for the Port
    pub liveness: Range,
    /// Bitwidth of the port
    pub bitwidth: Expr,
    /// Source position
    pos: GPosIdx,
}

impl PortDef {
    pub fn new(name: Id, liveness: Range, bitwidth: Expr) -> Self {
        Self {
            name,
            liveness,
            bitwidth,
            pos: GPosIdx::UNKNOWN,
        }
    }
}
impl Display for PortDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.liveness, self.name, self.bitwidth)
    }
}
impl WithPos for PortDef {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}
impl PortDef {
    /// Resolves all time expressions in this port definition
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        Self {
            liveness: self.liveness.resolve_event(bindings),
            ..self
        }
    }

    /// Resolves all width expressions in this port definition.
    /// Specifically:
    /// - The bitwidth of the port
    /// - The liveness condition
    pub fn resolve_offset(self, bindings: &Binding<Expr>) -> Self {
        Self {
            bitwidth: self.bitwidth.resolve(bindings),
            liveness: self.liveness.resolve_exprs(bindings),
            ..self
        }
    }
}

#[derive(Clone)]
pub struct InterfaceDef {
    /// Name of the port
    pub name: Id,
    /// Event that this port is an evidence of
    pub event: Id,
    // Position
    pos: GPosIdx,
}
impl Display for InterfaceDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "@interface[{}] {}: 1", self.event, self.name)
    }
}

impl InterfaceDef {
    pub fn new(name: Id, event: Id) -> Self {
        Self {
            name,
            event,
            pos: GPosIdx::UNKNOWN,
        }
    }
}

impl From<InterfaceDef> for PortDef {
    fn from(id: InterfaceDef) -> Self {
        let start = Time::from(id.event);
        let end = start.clone().increment(1.into());
        PortDef::new(id.name, Range::new(start, end), 1.into())
    }
}

impl WithPos for InterfaceDef {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}
