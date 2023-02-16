use super::Id;
use crate::utils;
use itertools::Itertools;
use std::fmt::Display;

#[derive(Default, Clone, PartialEq, Eq, Hash)]
/// An expression representing the sum of concrete numbers and abstract variables.
/// Comparison between two [Expr] does not take into account the order of the
/// variables.
pub struct Expr {
    // Concrete part of the time sum
    pub concrete: u64,
    // Abstract part of the time sum
    pub abs: Vec<Id>,
}

impl Expr {
    pub fn new(concrete: u64, abs: Vec<Id>) -> Self {
        Self {
            concrete,
            abs: abs.into_iter().unique().collect(),
        }
    }

    // Attempt to transform this in to a concrete time.
    // Panics if there are abstract values.
    pub fn concrete(&self) -> Option<u64> {
        if self.abs.is_empty() {
            Some(self.concrete)
        } else {
            None
        }
    }

    pub fn resolve(&self, bind: &utils::Binding<Expr>) -> Self {
        let mut offset = Expr {
            concrete: self.concrete,
            abs: vec![],
        };

        for x in &self.abs {
            if let Some(sum) = bind.find(x) {
                offset += sum.clone();
            }
        }

        offset
    }

    /// Check if this TimeSum is equal to 0
    pub fn is_zero(&self) -> bool {
        self.concrete == 0 && self.abs.is_empty()
    }
}

impl std::ops::Add for Expr {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            concrete: self.concrete + rhs.concrete,
            abs: self.abs.into_iter().chain(rhs.abs.into_iter()).collect(),
        }
    }
}

impl std::ops::AddAssign for Expr {
    fn add_assign(&mut self, rhs: Self) {
        self.concrete += rhs.concrete;
        self.abs.extend(rhs.abs);
    }
}

impl PartialOrd for Expr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let s1 = self.abs.iter().sorted().collect_vec();
        let s2 = other.abs.iter().sorted().collect_vec();
        if s1 == s2 {
            self.concrete.partial_cmp(&other.concrete)
        } else {
            None
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.abs
            .iter()
            .map(|t| t.to_string())
            .chain(
                std::iter::once(self.concrete)
                    .filter(|c| *c != 0)
                    .map(|t| t.to_string()),
            )
            .join("+")
            .fmt(f)
    }
}

impl From<Vec<u64>> for Expr {
    fn from(v: Vec<u64>) -> Self {
        Self {
            concrete: v.iter().sum(),
            abs: vec![],
        }
    }
}

impl From<u64> for Expr {
    fn from(v: u64) -> Self {
        Self {
            concrete: v,
            abs: vec![],
        }
    }
}
