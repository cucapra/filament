use std::collections::HashMap;

/// A representation of time
pub trait TimeRep
where
    Self: Sized,
{
    fn resolve(&self, bindings: &HashMap<super::Id, &Self>) -> Self;
}
