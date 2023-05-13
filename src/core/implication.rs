use std::fmt::Display;

use itertools::Itertools;

use crate::utils::{Binding, SExp};

use super::{expr::EvalBool, OrderConstraint, Expr};

/// A type representing the expression a => b
#[derive(Clone)]
pub struct Implication<T> {
    guard: Option<OrderConstraint<T>>,
    cons: OrderConstraint<T>
}

impl<T> Implication<T> {
    pub fn new(guard: OrderConstraint<T>, cons: OrderConstraint<T>) -> Self {
        Implication::new_opt(Some(guard), cons)
    }

    /// Creates a new Implication with an optional guard
    pub fn new_opt(guard: Option<OrderConstraint<T>>, cons: OrderConstraint<T>) -> Self {
        Implication {
            guard, cons
        }
    }
}

impl Implication<Expr> {
    pub fn resolve_expr(self, binding: &Binding<Expr>) -> Self {
        Implication {
            guard: self.guard.map(|g| g.resolve_expr(binding)),
            cons: self.cons.resolve_expr(binding)
        }
    }

    pub fn exprs(&self) -> Vec<&Expr> {
        match &self.guard {
            Some (g) => [g.exprs(), self.cons.exprs()].into_iter().flatten().collect_vec(),
            None => self.cons.exprs()
        }
    }
}

impl EvalBool for Implication<Expr> {
    fn resolve_bool(self, bind: &crate::utils::Binding<super::Expr>) -> crate::errors::FilamentResult<bool> {
        Ok (
            match self.guard {
                Some(g) => !g.resolve_bool(bind)? || self.cons.resolve_bool(bind)?,
                None => self.cons.resolve_bool(bind)?
            }
        )
    }
}


impl<T> From<Implication<T>> for SExp
where
    SExp: From<OrderConstraint<T>>,
{
    fn from(c: Implication<T>) -> Self {
        match c.guard {
            // If g then c.cons else true
            Some(g) => SExp(format!("(ite {} {} true)", SExp::from(g), SExp::from(c.cons))),
            // no guard
            None => SExp::from(c.cons)
        }
    }
}

impl<T> Display for Implication<T>
where
    T: Display
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.guard {
            Some(g) => write!(f, "{} => {}", g, self.cons),
            None => write!(f, "{}", self.cons)
        }
        
    }
}

impl<T> From<OrderConstraint<T>> for Implication<T> {
    fn from(cons: OrderConstraint<T>) -> Self {
        Implication::new_opt(None, cons)
    }
}