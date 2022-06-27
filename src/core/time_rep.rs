use std::{collections::HashMap, fmt::Display};

/// A representation of time
pub trait TimeRep
where
    Self: Sized + Clone + Display,
{
    fn unit(event: super::Id, state: u64) -> Self;
    fn increment(self, n: u64) -> Self;
    fn resolve(&self, bindings: &HashMap<super::Id, &Self>) -> Self;
}
