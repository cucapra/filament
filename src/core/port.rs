use super::{Id, Range, Time};
use crate::utils::Binding;
use crate::{errors::WithPos, utils::GPosIdx};
use std::fmt::Display;

/// An expression that can represent either constants or variables
#[derive(Hash, Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum Expr {
    /// A constant
    Const(u64),
    /// A parameter
    Var(Id),
}

impl Expr {
    pub fn concrete(&self) -> u64 {
        match self {
            Expr::Const(c) => *c,
            Expr::Var(_) => {
                unreachable!("Cannot convert {} into concrete value", self)
            }
        }
    }
    pub fn resolve(&self, bindings: &Binding<Expr>) -> Option<Expr> {
        match self {
            Expr::Const(_) => Some(self.clone()),
            Expr::Var(v) => bindings.find(v).cloned(),
        }
    }
}

impl From<Id> for Expr {
    fn from(v: Id) -> Self {
        Self::Var(v)
    }
}
impl From<u64> for Expr {
    fn from(v: u64) -> Self {
        Self::Const(v)
    }
}
impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Const(c) => write!(f, "{c}"),
            Expr::Var(v) => write!(f, "{v}"),
        }
    }
}

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
    pub fn resolve_event(&self, bindings: &Binding<Time>) -> Self {
        Self {
            liveness: self.liveness.resolve_event(bindings),
            ..(self.clone())
        }
    }

    /// Resolves all width expressions in this port definition.
    /// Specifically:
    /// - The bitwidth of the port
    /// - The liveness condition
    pub fn resolve_offset(&self, bindings: &Binding<Expr>) -> Self {
        Self {
            bitwidth: self.bitwidth.resolve(bindings).unwrap(),
            liveness: self.liveness.resolve_offset(bindings),
            ..(self.clone())
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
        let start = Time::unit(id.event, 0);
        let end = start.clone().increment(Expr::Const(1));
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
