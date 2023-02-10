use itertools::Itertools;

use super::{Id, PortParam, Range, TimeRep, TimeSub};
use crate::interval_checking::SExp;
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash)]
/// An opaque time sum expression
pub struct TimeSum<T>(Vec<T>);

impl TimeSum<u64> {
    pub fn concrete(&self) -> u64 {
        self.0[0]
    }
}

impl<T> From<Vec<T>> for TimeSum<T> {
    fn from(v: Vec<T>) -> Self {
        Self(v)
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
/// Represents expression of the form `G+1+k`
pub struct Time<Offset: Clone> {
    /// The event for the time expression
    pub event: Id,
    /// The offsets for the time expression
    offset: TimeSum<Offset>,
}

impl<T: Clone> Time<T> {
    pub fn new(event: Id, offset: Vec<T>) -> Self {
        Self {
            event,
            offset: offset.into(),
        }
    }

    pub fn offset(&self) -> &TimeSum<T> {
        &self.offset
    }
}

impl<T: Clone> From<Time<T>> for SExp
where
    SExp: From<T>,
{
    fn from(t: Time<T>) -> SExp {
        SExp(format!(
            "(+ {} {})",
            t.event,
            t.offset
                .0
                .clone()
                .into_iter()
                .map(|e| SExp::from(e).to_string())
                .collect_vec()
                .join(" ")
        ))
    }
}

impl<T: Clone> Range<Time<T>>
where
    Time<T>: TimeRep,
{
    /// Convert this interval into an offset. Only possible when interval uses
    /// exactly one event for both start and end.
    pub fn as_offset(&self) -> Option<(Id, TimeSum<T>, TimeSum<T>)> {
        let Range { start, end, .. } = &self;
        if start.event == end.event {
            Some((
                start.event.clone(),
                start.offset().clone(),
                end.offset().clone(),
            ))
        } else {
            None
        }
    }
}

// Implementation for concrete time representation
impl TimeSub<Time<u64>> {
    /// Attempt to automatically simplify the difference when possible
    pub fn build(l: Time<u64>, r: Time<u64>) -> Self {
        if l.event == r.event {
            // Guaranteed to only have one offset
            let l = l.offset().0[0];
            let r = r.offset().0[0];
            if l > r {
                return Self::Unit(l - r);
            } else {
                return Self::Unit(r - l);
            }
        }
        Self::Sym { l, r }
    }
}

impl Display for Time<u64> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.event)?;
        if self.offset.0[0] != 0 {
            write!(f, "+{}", self.offset.0[0])?;
        }
        Ok(())
    }
}

impl TimeRep for Time<u64> {
    type SubRep = TimeSub<Self>;
    type Offset = u64;

    fn unit(event: Id, offset: u64) -> Self {
        Self {
            event,
            offset: vec![offset].into(),
        }
    }

    fn increment(mut self, n: u64) -> Self {
        self.offset.0[0] += n;
        self
    }

    fn resolve_event(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.0[0] += self.offset.0[0];
        n
    }

    fn resolve_offset(&self, _: &super::Binding<Self::Offset>) -> Self {
        self.clone()
    }

    fn sub(self, other: Self) -> Self::SubRep {
        TimeSub::build(self, other)
    }

    fn event(&self) -> Id {
        self.event.clone()
    }
}

impl Display for Time<PortParam> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.event)?;
        for x in &self.offset.0 {
            write!(f, "+{x}")?;
        }
        Ok(())
    }
}

impl From<Time<PortParam>> for SExp {
    fn from(value: Time<PortParam>) -> Self {
        SExp(format!(
            "(+ {} {})",
            value.event,
            value
                .offset
                .0
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        ))
    }
}

impl TimeRep for Time<PortParam> {
    type SubRep = TimeSub<Self>;
    type Offset = PortParam;

    fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: vec![PortParam::Const(state)].into(),
        }
    }

    fn increment(mut self, n: u64) -> Self {
        self.offset.0.push(PortParam::Const(n));
        self
    }

    fn resolve_event(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.0.extend(self.offset.0.clone());
        n
    }

    fn resolve_offset(&self, bindings: &super::Binding<Self::Offset>) -> Self {
        let mut offset = Vec::with_capacity(self.offset.0.len());

        for x in &self.offset.0 {
            match x {
                PortParam::Const(x) => offset.push(PortParam::Const(*x)),
                PortParam::Var(x) => offset.push(bindings.get(x).clone()),
            }
        }

        Self {
            event: self.event.clone(),
            offset: offset.into(),
        }
    }

    fn sub(self, other: Self) -> Self::SubRep {
        TimeSub::sym(self, other)
    }

    fn event(&self) -> Id {
        self.event.clone()
    }
}
