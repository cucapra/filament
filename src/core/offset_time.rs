use super::{Id, PortParam, Range, TimeRep, TimeSub};
use crate::interval_checking::SExp;
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash)]
/// Represents expression of the form `G+1+k`
pub struct Time<Offset: Clone> {
    /// The event for the time expression
    pub event: Id,
    /// The offsets for the time expression
    offset: Offset,
}

impl<T: Clone> Time<T> {
    pub fn offset(&self) -> &T {
        &self.offset
    }
}

impl<T: Clone> From<Time<T>> for SExp
where
    SExp: From<T>,
{
    fn from(t: Time<T>) -> SExp {
        SExp(format!("(+ {} {})", t.event, SExp::from(t.offset)))
    }
}

impl<T: Clone> Range<Time<T>>
where
    Time<T>: TimeRep,
{
    /// Convert this interval into an offset. Only possible when interval uses
    /// exactly one event for both start and end.
    pub fn as_offset(&self) -> Option<(Id, T, T)> {
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
            let l = l.offset();
            let r = r.offset();
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
        if self.offset != 0 {
            write!(f, "+{}", self.offset)?;
        }
        Ok(())
    }
}

impl TimeRep for Time<u64> {
    type SubRep = TimeSub<Self>;

    fn unit(event: Id, offset: u64) -> Self {
        Self { event, offset }
    }

    fn increment(mut self, n: u64) -> Self {
        self.offset += n;
        self
    }

    fn resolve(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset += self.offset;
        n
    }

    fn sub(self, other: Self) -> Self::SubRep {
        TimeSub::build(self, other)
    }

    fn event(&self) -> Id {
        self.event.clone()
    }
}

#[derive(Clone, Eq, Hash, PartialEq)]
/// Implementation of parameteric time
pub struct ParamTime(Vec<PortParam>);

impl Display for Time<ParamTime> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.event)?;
        for x in &self.offset.0 {
            write!(f, "+{x}")?;
        }
        Ok(())
    }
}

impl From<Time<ParamTime>> for SExp {
    fn from(value: Time<ParamTime>) -> Self {
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

impl TimeRep for Time<ParamTime> {
    type SubRep = TimeSub<Self>;

    fn unit(event: Id, state: u64) -> Self {
        Time {
            event,
            offset: ParamTime(vec![PortParam::Const(state)]),
        }
    }

    fn increment(mut self, n: u64) -> Self {
        self.offset.0.push(PortParam::Const(n));
        self
    }

    fn resolve(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.0.extend(self.offset.0.clone());
        n
    }

    fn sub(self, other: Self) -> Self::SubRep {
        TimeSub::sym(self, other)
    }

    fn event(&self) -> Id {
        self.event.clone()
    }
}
