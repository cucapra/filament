use crate::core;

use super::SExp;

/// A Fact in the Context
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Fact {
    Constraint(core::Constraint),
    Interval(IntervalFact),
}
impl Fact {
    /// Construct a [IntervalFact] with `tag` set to [FactType::Equality].
    pub fn equality(left: core::Interval, right: core::Interval) -> Self {
        Self::Interval(IntervalFact {
            tag: FactType::Equality,
            left,
            right,
        })
    }

    /// Construct a [IntervalFact] with `tag` set to [FactType::Subset].
    pub fn subset(left: core::Interval, right: core::Interval) -> Self {
        Self::Interval(IntervalFact {
            tag: FactType::Subset,
            left,
            right,
        })
    }
}
impl std::fmt::Debug for Fact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fact::Constraint(con) => write!(f, "{:?}", con),
            Fact::Interval(interval) => write!(f, "{:?}", interval),
        }
    }
}
impl From<&Fact> for SExp {
    fn from(f: &Fact) -> Self {
        match f {
            Fact::Constraint(con) => SExp::from(con),
            Fact::Interval(interval) => SExp::from(interval),
        }
    }
}

/// Type of the fact
#[derive(Debug, Hash, Eq, PartialEq, Clone)]
pub enum FactType {
    /// Represents set equality
    Equality,
    /// Represents subset
    Subset,
}

/// Known interval fact
#[derive(Hash, Eq, PartialEq, Clone)]
pub struct IntervalFact {
    pub tag: FactType,
    pub left: core::Interval,
    pub right: core::Interval,
}
impl std::fmt::Debug for IntervalFact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let core::Interval { start, end, .. } = &self.left;
        write!(f, "[")?;
        start.fmt(f)?;
        write!(f, ", ")?;
        end.fmt(f)?;
        write!(f, "]")?;
        match self.tag {
            FactType::Equality => write!(f, " == ")?,
            FactType::Subset => write!(f, " ⊆ ")?,
        }
        let core::Interval { start, end, .. } = &self.right;
        write!(f, "[")?;
        start.fmt(f)?;
        write!(f, ", ")?;
        end.fmt(f)?;
        write!(f, "]")
    }
}

impl From<&IntervalFact> for super::SExp {
    fn from(f: &IntervalFact) -> Self {
        match f.tag {
            FactType::Equality => {
                todo!("Converting equality facts into z3 asserts")
            }
            FactType::Subset => {
                let core::Interval { start: ls, end: le } = &f.left;
                let core::Interval { start: rs, end: re } = &f.right;
                super::SExp(format!(
                    "(and (<= {} {}) (>= {} {}))",
                    super::SExp::from(rs),
                    super::SExp::from(ls),
                    super::SExp::from(re),
                    super::SExp::from(le)
                ))
            }
        }
    }
}
