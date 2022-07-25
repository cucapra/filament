use super::{Binding, Id, Interval, TimeRep, TimeSub, WithTime};
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
    fn resolve(&self, bindings: &Binding<T>) -> Self {
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
    pub delay: TimeSub<T>,
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
            "@interface<{}, {}> {}: 1",
            self.event, self.delay, self.name
        )
    }
}

impl<T> InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    pub fn new(name: Id, event: Id, delay: TimeSub<T>) -> Self
    where
        T: TimeRep + Clone,
    {
        Self {
            name,
            event,
            delay,
            pos: None,
        }
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
