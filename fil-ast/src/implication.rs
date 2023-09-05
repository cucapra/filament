use super::{Binding, Expr, OrderConstraint};
use itertools::Itertools;

/// A type representing the expression a => b
#[derive(Clone)]
pub struct Implication<T> {
    pub guard: Option<OrderConstraint<T>>,
    pub cons: OrderConstraint<T>,
}

impl<T> Implication<T>
where
    T: Clone,
{
    /// Creates an [Implication] `guard => cons`
    pub fn implies(
        guard: OrderConstraint<T>,
        cons: OrderConstraint<T>,
    ) -> Self {
        Implication::new_opt(Some(guard), cons)
    }

    /// Creates an [Implication] with no guard, something that must always be true
    pub fn fact(cons: OrderConstraint<T>) -> Self {
        Implication::new_opt(None, cons)
    }

    /// Creates a pair of [Implication]s representing an iff (`a <=> b`) clause
    pub fn iff(
        left: OrderConstraint<T>,
        right: OrderConstraint<T>,
    ) -> [Self; 2] {
        [
            Implication::implies(left.clone(), right.clone()),
            Implication::implies(right, left),
        ]
    }

    /// Creates a new [Implication] with an optional guard
    fn new_opt(
        guard: Option<OrderConstraint<T>>,
        cons: OrderConstraint<T>,
    ) -> Self {
        Implication { guard, cons }
    }
}

impl Implication<Expr> {
    pub fn resolve_expr(self, binding: &Binding<Expr>) -> Self {
        Implication {
            guard: self.guard.map(|g| g.resolve_expr(binding)),
            cons: self.cons.resolve_expr(binding),
        }
    }

    pub fn exprs(&self) -> Vec<&Expr> {
        match &self.guard {
            Some(g) => [g.exprs(), self.cons.exprs()]
                .into_iter()
                .flatten()
                .collect_vec(),
            None => self.cons.exprs(),
        }
    }
}

impl<T> From<OrderConstraint<T>> for Implication<T>
where
    T: Clone,
{
    fn from(cons: OrderConstraint<T>) -> Self {
        Implication::fact(cons)
    }
}
