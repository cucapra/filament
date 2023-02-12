use itertools::Itertools;

use super::{Id, PortParam, Range, TimeRep, TimeSub};
use crate::interval_checking::SExp;
use std::fmt::Display;

#[derive(Clone, PartialEq, Eq, Hash)]
/// An opaque time sum expression
pub struct TimeSum<T> {
    concrete: u64,
    abs: Vec<T>,
}

impl<T> TimeSum<T> {
    pub fn concrete(&self) -> u64 {
        assert!(self.abs.is_empty());
        self.concrete
    }
}

impl From<Vec<u64>> for TimeSum<u64> {
    fn from(v: Vec<u64>) -> Self {
        Self {
            concrete: v.iter().sum(),
            abs: vec![],
        }
    }
}

impl From<Vec<PortParam>> for TimeSum<PortParam> {
    fn from(v: Vec<PortParam>) -> Self {
        let mut ts = Self {
            concrete: 0,
            abs: vec![],
        };
        for p in v {
            match p {
                PortParam::Const(c) => ts.concrete += c,
                PortParam::Var(_) => ts.abs.push(p),
            }
        }
        ts
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
    pub fn offset(&self) -> &TimeSum<T> {
        &self.offset
    }

    pub fn new(event: Id, offset: Vec<T>) -> Self
    where
        TimeSum<T>: From<Vec<T>>,
    {
        Self {
            event,
            offset: offset.into(),
        }
    }
}


impl<T: Clone> From<Time<T>> for SExp
where
    SExp: From<T>,
{
    fn from(t: Time<T>) -> SExp {
        if t.offset.abs.is_empty() && t.offset.concrete == 0 {
            SExp(format!("{}", t.event))
        } else if t.offset.abs.is_empty() {
            SExp(format!("(+ {} {})", t.event, t.offset.concrete))
        } else {
            SExp(format!(
                "(+ {} {} {})",
                t.event,
                t.offset.concrete,
                t.offset
                    .abs
                    .clone()
                    .into_iter()
                    .map(|e| SExp::from(e).to_string())
                    .collect_vec()
                    .join(" ")
            ))
        }
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
            // Guaranteed to only have concrete part.
            let l = l.offset().concrete;
            let r = r.offset().concrete;
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
        if self.offset.concrete() != 0 {
            write!(f, "+{}", self.offset.concrete())?;
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
        self.offset.concrete += n;
        self
    }

    fn resolve_event(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.concrete += self.offset.concrete;
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

    fn lift(self) -> Time<PortParam> {
        Time {
            event: self.event,
            offset: vec![PortParam::Const(self.offset.concrete)].into(),
        }
    }
}

impl Display for Time<PortParam> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.event)?;
        for x in &self.offset.abs {
            write!(f, "+{x}")?;
        }
        if self.offset.concrete != 0 {
            write!(f, "+{}", self.offset.concrete)?;
        }
        Ok(())
    }
}

impl From<Time<PortParam>> for SExp {
    fn from(value: Time<PortParam>) -> Self {
        SExp(format!(
            "(+ {} {} {})",
            value.event,
            value.offset.concrete,
            value
                .offset
                .abs
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
        self.offset.concrete += n;
        self
    }

    fn resolve_event(&self, bindings: &super::Binding<Self>) -> Self {
        let mut n = bindings.get(&self.event).clone();
        n.offset.concrete += self.offset.concrete;
        n.offset.abs.extend(self.offset.abs.clone());
        n
    }

    fn resolve_offset(&self, bindings: &super::Binding<Self::Offset>) -> Self {
        let mut offset = TimeSum {
            concrete: self.offset.concrete,
            abs: vec![],
        };

        for x in &self.offset.abs {
            match x {
                PortParam::Const(_) => unreachable!("Representation Invariant: abs should only contain PortParam::Var"),
                PortParam::Var(x) => match bindings.find(x) {
                    Some(PortParam::Const(c)) => offset.concrete += c,
                    Some(PortParam::Var(v)) => offset.abs.push(PortParam::Var(v.clone())),
                    None => offset.abs.push(PortParam::Var(x.clone())),
                }
            }
        }

        Time {
            event: self.event.clone(),
            offset,
        }
    }

    fn sub(self, other: Self) -> Self::SubRep {
        build_param_sub(self, other)
    }

    fn event(&self) -> Id {
        self.event.clone()
    }

    fn lift(self) -> Time<PortParam> {
        self
    }
}

/// Attempt to automatically simplify the difference when possible
fn build_param_sub(
    l: Time<PortParam>,
    r: Time<PortParam>,
) -> TimeSub<Time<PortParam>> {
    let lc = l.offset().concrete;
    let rc = r.offset().concrete;
    if l.event == r.event && l.offset().abs == r.offset().abs {
        TimeSub::Unit(u64::abs_diff(lc, rc))
    } else {
        // Only add abstract variable when neither side has it.
        let mut abs = vec![];
        for a in &l.offset().abs {
            if !r.offset().abs.contains(a) {
                abs.push(a.clone());
            }
        }

        // If the left side has more concrete time, then the right side
        // is the one that is subtracted.
        if lc > rc {
            TimeSub::Sym {
                l: Time {
                    event: l.event,
                    offset: TimeSum {
                        concrete: lc - rc,
                        abs,
                    },
                },
                r: Time::unit(r.event, 0),
            }
        } else {
            TimeSub::Sym {
                l: Time::unit(l.event, 0),
                r: Time {
                    event: r.event,
                    offset: TimeSum {
                        concrete: rc - lc,
                        abs,
                    },
                },
            }
        }
    }
}
