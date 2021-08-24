use crate::core;

/// Type of the fact
#[derive(Debug, Hash, Eq, PartialEq)]
pub enum FactType {
    /// Represents set equality
    Equality,
    /// Represents subset
    Subset,
}

/// Set of known interval facts and equalities.
#[derive(Hash, Eq, PartialEq)]
pub struct Fact {
    pub tag: FactType,
    pub left: core::Interval,
    pub right: core::Interval,
}
impl Fact {
    /// Construct a [Fact] with `tag` set to [FactType::Equality].
    pub fn equality(left: core::Interval, right: core::Interval) -> Self {
        Fact {
            tag: FactType::Equality,
            left,
            right,
        }
    }

    /// Construct a [Fact] with `tag` set to [FactType::Subset].
    pub fn subset(left: core::Interval, right: core::Interval) -> Self {
        Fact {
            tag: FactType::Subset,
            left,
            right,
        }
    }
}
impl std::fmt::Debug for Fact {
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
