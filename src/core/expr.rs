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
            abs: abs.into_iter().collect(),
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

    /// Resolve this expression using the given binding for abstract variables.
    pub fn resolve(&self, bind: &utils::Binding<Expr>) -> Self {
        let mut offset = Expr {
            concrete: self.concrete,
            abs: vec![],
        };

        for x in &self.abs {
            if let Some(sum) = bind.find(x) {
                offset += sum.clone();
            } else {
                offset.abs.push(x.clone());
            }
        }

        offset
    }

    /// Check if this TimeSum is equal to 0
    pub fn is_zero(&self) -> bool {
        self.concrete == 0 && self.abs.is_empty()
    }

    /// Get all the abstract variables in this expression
    pub fn exprs(&self) -> &Vec<Id> {
        &self.abs
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

impl std::ops::Sub for Expr {
    /// Return the difference and optionally the "residual" if the subtraction needs to
    /// represent some concrete or abstract values not on the left hand hide.
    /// For example:
    ///   L+1 - (W+2) => (L, Some(W+1))
    type Output = (Expr, Option<Expr>);

    fn sub(self, rhs: Self) -> Self::Output {
        // If the LHS concrete is bigger, then there is no residual
        let (concrete, residual) = if self.concrete >= rhs.concrete {
            (self.concrete - rhs.concrete, 0)
        } else {
            (0, rhs.concrete - self.concrete)
        };

        // Each time a variable occurs in the RHS, remove it from the LHS
        let mut left = self.abs;
        let mut right = vec![];
        for x in rhs.abs {
            if let Some(i) = left.iter().position(|y| *y == x) {
                left.remove(i);
            } else {
                right.push(x);
            }
        }
        if right.is_empty() && residual == 0 {
            (Expr::new(concrete, left), None)
        } else {
            (Expr::new(concrete, left), Some(Expr::new(residual, right)))
        }
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
        if self.abs.is_empty() {
            write!(f, "{}", self.concrete)
        } else {
            self.abs
                .iter()
                .map(|t| format!("#{t}"))
                .chain(
                    std::iter::once(self.concrete)
                        .filter(|c| *c != 0)
                        .map(|t| t.to_string()),
                )
                .join("+")
                .fmt(f)
        }
    }
}

impl From<Expr> for utils::SExp {
    fn from(value: Expr) -> Self {
        utils::SExp(format!(
            "(+ {} {})",
            value.abs.iter().map(|t| t.as_ref()).join(" "),
            value.concrete
        ))
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

impl From<Id> for Expr {
    fn from(v: Id) -> Self {
        Self {
            concrete: 0,
            abs: vec![v],
        }
    }
}
