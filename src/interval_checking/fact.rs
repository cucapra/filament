use std::iter;

use crate::core;

use super::SExp;

/// A Fact in the Context
#[derive(Hash, Eq, PartialEq, Clone)]
pub enum Fact {
    Constraint(core::Constraint<core::FsmIdxs>),
    Interval(IntervalFact),
}
impl Fact {
    pub fn constraint(
        cons: core::Constraint<core::FsmIdxs>,
    ) -> impl Iterator<Item = Fact> {
        iter::once(Fact::Constraint(cons))
    }

    /// Construct a [IntervalFact] with `tag` set to [FactType::Equality].
    pub fn equality(
        left: core::Range<super::TimeRep>,
        right: core::Range<super::TimeRep>,
    ) -> impl Iterator<Item = Fact> {
        iter::once(Self::Interval(IntervalFact {
            tag: FactType::Equality,
            left,
            right,
        }))
    }

    /// Construct a [IntervalFact] with `tag` set to [FactType::Subset].
    pub fn subset(
        left: core::Range<super::TimeRep>,
        right: core::Range<super::TimeRep>,
    ) -> impl Iterator<Item = Fact> {
        iter::once(Self::Interval(IntervalFact {
            tag: FactType::Subset,
            left,
            right,
        }))
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
    pub left: core::Range<super::TimeRep>,
    pub right: core::Range<super::TimeRep>,
}
impl std::fmt::Debug for IntervalFact {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let core::Range { start, end } = &self.left;
        write!(f, "[")?;
        start.fmt(f)?;
        write!(f, ", ")?;
        end.fmt(f)?;
        write!(f, "]")?;
        match self.tag {
            FactType::Equality => write!(f, " == ")?,
            FactType::Subset => write!(f, " ⊆ ")?,
        }
        let core::Range { start, end } = &self.right;
        write!(f, "[")?;
        start.fmt(f)?;
        write!(f, ", ")?;
        end.fmt(f)?;
        write!(f, "]")
    }
}

impl From<&IntervalFact> for super::SExp {
    fn from(f: &IntervalFact) -> Self {
        let core::Range { start: ls, end: le } = &f.left;
        let core::Range { start: rs, end: re } = &f.right;
        let rs_sexp: SExp = rs.into();
        let ls_sexp: SExp = ls.into();
        let re_sexp: SExp = re.into();
        let le_sexp: SExp = le.into();
        match f.tag {
            FactType::Equality => super::SExp(format!(
                "(and (= {rs_sexp} {ls_sexp}) (= {re_sexp} {le_sexp}))",
            )),
            FactType::Subset => super::SExp(format!(
                "(and (<= {rs_sexp} {ls_sexp}) (>= {re_sexp} {le_sexp}))",
            )),
        }
    }
}
