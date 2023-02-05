use super::{Binding, Id, Interval, Range, Time, TimeRep, TimeSub, WithTime};
use crate::errors::{self, WithPos};
use std::fmt::Display;

#[derive(Clone)]
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
    T: Clone + TimeRep,
    W: Clone,
{
    /// Name of the port
    pub name: Id,
    /// Liveness condition for the Port
    pub liveness: Interval<T>,
    /// Bitwidth of the port
    pub bitwidth: W,
    /// Source position
    pos: Option<errors::Span>,
}

impl<T, W> PortDef<T, W>
where
    T: Clone + TimeRep,
    W: Clone,
{
    pub fn new(name: Id, liveness: Interval<T>, bitwidth: W) -> Self {
        Self {
            name,
            liveness,
            bitwidth,
            pos: None,
        }
    }
}
impl<T, W> Display for PortDef<T, W>
where
    T: Display + Clone + TimeRep,
    W: Display + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.liveness, self.name, self.bitwidth)
    }
}
impl<T, W> WithPos for PortDef<T, W>
where
    T: TimeRep,
    W: Clone,
{
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}
impl<T, W> WithTime<T> for PortDef<T, W>
where
    W: Clone,
    T: Clone + TimeRep,
{
    fn resolve(&self, bindings: &Binding<T>) -> Self {
        Self {
            liveness: self.liveness.resolve(bindings),
            ..(self.clone())
        }
    }
}

#[derive(Clone)]
pub struct InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    /// Name of the port
    pub name: Id,
    /// Event that this port is an evidence of
    pub event: Id,
    /// End event
    pub end: T,
    /// Liveness of the interface signal
    pub(super) liveness: Interval<T>,
    /// Port only used for modeling purposes and does not need to be connected
    pub phantom: bool,
    // Position
    pos: Option<errors::Span>,
}
impl<T> Display for InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let phan = if self.phantom { " @phantom" } else { "" };
        write!(
            f,
            "@interface[{}, {}]{} {}: 1",
            self.event, self.end, phan, self.name
        )
    }
}

impl<T> InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    pub fn new(name: Id, event: Id, end: T, phantom: bool) -> Self
    where
        T: TimeRep + Clone,
    {
        let start = T::unit(event.clone(), 0);
        let liveness = Interval::from(Range::new(start.clone(), end.clone()))
            .with_exact(Range::new(start.clone(), start.increment(1)));
        Self {
            name,
            end,
            event,
            liveness,
            phantom,
            pos: None,
        }
    }
}

impl InterfaceDef<Time<u64>> {
    /// Attempts to return a concrete delay for this interface. Panics if the
    /// end time is a max-expression or uses different time variables
    pub fn delay(&self) -> TimeSub<Time<u64>> {
        Time::unit(self.event.clone(), 0).sub(self.end.clone())
    }
}

impl<T> WithPos for InterfaceDef<T>
where
    T: TimeRep,
{
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}
