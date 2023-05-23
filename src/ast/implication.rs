use super::{expr::EvalBool, Expr, OrderConstraint};
use crate::{
    errors::FilamentResult,
    utils::{self, Binding, SExp},
};
use itertools::Itertools;
use std::fmt::Display;

/// A type representing the expression a => b
#[derive(Clone)]
pub struct Implication<T> {
    pub guard: Option<OrderConstraint<T>>,
    pub cons: OrderConstraint<T>,
}

impl<T> Implication<T> {
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

    /// Creates a new [Implication] with an optional guard
    fn new_opt(
        guard: Option<OrderConstraint<T>>,
        cons: OrderConstraint<T>,
    ) -> Self {
        Implication { guard, cons }
    }
}

impl<T> Implication<T>
where
    SExp: From<Implication<T>>,
{
    pub fn obligation<S: ToString>(self, reason: S) -> utils::Obligation {
        utils::Obligation::new(SExp::from(self), reason.to_string())
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

impl EvalBool for Implication<Expr> {
    fn resolve_bool(self, bind: &Binding<Expr>) -> FilamentResult<bool> {
        Ok(match self.guard {
            Some(g) => {
                !g.resolve_bool(bind)? || self.cons.resolve_bool(bind)?
            }
            None => self.cons.resolve_bool(bind)?,
        })
    }
}

impl<T> From<Implication<T>> for SExp
where
    SExp: From<OrderConstraint<T>>,
{
    fn from(c: Implication<T>) -> Self {
        match c.guard {
            Some(g) => {
                SExp(format!("(=> {} {})", SExp::from(g), SExp::from(c.cons)))
            }
            // no guard
            None => SExp::from(c.cons),
        }
    }
}

impl<T> Display for Implication<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.guard {
            Some(g) => write!(f, "{} => {}", g, self.cons),
            None => write!(f, "{}", self.cons),
        }
    }
}

impl<T> From<OrderConstraint<T>> for Implication<T> {
    fn from(cons: OrderConstraint<T>) -> Self {
        Implication::fact(cons)
    }
}
