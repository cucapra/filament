use super::{Binding, Id, Range, TimeRep, WidthRep, WithTime};
use crate::{errors::WithPos, utils::GPosIdx};
use std::fmt::Display;

#[derive(Hash, Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub enum PortParam {
    /// A constant
    Const(u64),
    /// A parameter
    Var(Id),
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
pub struct PortDef<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    /// Name of the port
    pub name: Id,
    /// Liveness condition for the Port
    pub liveness: Range<T>,
    /// Bitwidth of the port
    pub bitwidth: W,
    /// Source position
    pos: GPosIdx,
}

impl<T, W> PortDef<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    pub fn new(name: Id, liveness: Range<T>, bitwidth: W) -> Self {
        Self {
            name,
            liveness,
            bitwidth,
            pos: GPosIdx::UNKNOWN,
        }
    }
}
impl<T, W> Display for PortDef<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.liveness, self.name, self.bitwidth)
    }
}
impl<T, W> WithPos for PortDef<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}
impl<T, W> WithTime<T> for PortDef<T, W>
where
    W: WidthRep,
    T: TimeRep,
{
    fn resolve_event(&self, bindings: &Binding<T>) -> Self {
        Self {
            liveness: self.liveness.resolve_event(bindings),
            ..(self.clone())
        }
    }

    fn resolve_offset(
        &self,
        bindings: &Binding<<T as TimeRep>::Offset>,
    ) -> Self {
        Self {
            liveness: self.liveness.resolve_offset(bindings),
            ..(self.clone())
        }
    }

    fn events(&self) -> Vec<Id> {
        todo!("events for PortDef")
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

impl WithPos for InterfaceDef {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}
