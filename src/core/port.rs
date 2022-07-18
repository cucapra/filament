use super::{FsmIdxs, Id, Interval, Range, TimeRep, TimeSub, WithTime};
use crate::errors::{self, WithPos};
use std::fmt::Display;

#[derive(Clone)]
pub struct PortDef<T>
where
    T: Clone + TimeRep,
{
    /// Name of the port
    pub name: Id,
    /// Liveness condition for the Port
    pub liveness: Interval<T>,
    /// Bitwidth of the port
    pub bitwidth: u64,
    /// Source position
    pos: Option<errors::Span>,
}

impl<T> PortDef<T>
where
    T: Clone + TimeRep,
{
    pub fn new(name: Id, liveness: Interval<T>, bitwidth: u64) -> Self {
        Self {
            name,
            liveness,
            bitwidth,
            pos: None,
        }
    }
}
impl<T> Display for PortDef<T>
where
    T: Display + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.liveness, self.name, self.bitwidth)
    }
}
impl<T> WithPos for PortDef<T>
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
impl<T: Clone + TimeRep> WithTime<T> for PortDef<T> {
    fn resolve(
        &self,
        bindings: &std::collections::HashMap<super::Id, &T>,
    ) -> Self {
        Self {
            liveness: self.liveness.resolve(bindings),
            ..self.clone()
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
    // Event that this port is an evidence of
    pub event: Id,
    // End event
    pub end: T,
    // Liveness of the interface signal
    pub(super) liveness: Interval<T>,
    // Position
    pos: Option<errors::Span>,
}
impl<T> Display for InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@interface[{}, {}] {}: 1",
            self.event, self.end, self.name
        )
    }
}

impl<T> InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    pub fn new(name: Id, event: Id, end: T) -> Self
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
            pos: None,
        }
    }
}

impl InterfaceDef<FsmIdxs> {
    /// Attempts to return a concrete delay for this interface. Panics if the
    /// end time is a max-expression or uses different time variables
    pub fn delay(&self) -> TimeSub<FsmIdxs> {
        FsmIdxs::unit(self.event.clone(), 0) - self.end.clone()
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
