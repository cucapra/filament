use std::collections::HashMap;

/// A representation of time
pub trait TimeRep
where
    Self: Sized,
{
    fn unit(event: super::Id, state: u64) -> Self;
    fn increment(self, n: u64) -> Self;
    fn resolve(&self, bindings: &HashMap<super::Id, &Self>) -> Self;
}
