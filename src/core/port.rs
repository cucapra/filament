use super::{Binding, ConcTime, Id, Range};
use crate::{errors::WithPos, utils::GPosIdx};
use std::fmt::Display;

#[derive(Hash, Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum PortParam {
    /// A constant
    Const(u64),
    /// A parameter
    Var(Id),
}

/// Representation of widths in the program
pub type Width = PortParam;

impl PortParam {
    pub fn concrete(&self) -> u64 {
        match self {
            PortParam::Const(c) => *c,
            PortParam::Var(_) => {
                unreachable!("Cannot convert {} into concrete value", self)
            }
        }
    }
    pub fn resolve(&self, bindings: &Binding<Width>) -> Option<Width> {
        match self {
            PortParam::Const(_) => Some(self.clone()),
            PortParam::Var(v) => bindings.find(v).cloned(),
        }
    }
}

impl From<Id> for PortParam {
    fn from(v: Id) -> Self {
        Self::Var(v)
    }
}
impl From<u64> for PortParam {
    fn from(v: u64) -> Self {
        Self::Const(v)
    }
}
impl Display for PortParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortParam::Const(c) => write!(f, "{c}"),
            PortParam::Var(v) => write!(f, "{v}"),
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
    pub bitwidth: Width,
    /// Source position
    pos: GPosIdx,
}

impl PortDef {
    pub fn new(name: Id, liveness: Range, bitwidth: Width) -> Self {
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
    pub fn resolve_event(&self, bindings: &Binding<ConcTime>) -> Self {
        Self {
            liveness: self.liveness.resolve_event(bindings),
            ..(self.clone())
        }
    }

    /// Resolves all width expressions in this port definition.
    /// Specifically:
    /// - The bitwidth of the port
    /// - The liveness condition
    pub fn resolve_offset(&self, bindings: &Binding<Width>) -> Self {
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
        let start = ConcTime::unit(id.event, 0);
        let end = start.clone().increment(PortParam::Const(1));
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
