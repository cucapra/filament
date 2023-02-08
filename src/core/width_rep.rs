use super::PortParam;

/// Representation of a width in the program
pub trait WidthRep
where
    Self: Clone + Eq + Ord + std::fmt::Display,
{
}

impl WidthRep for u64 {}
impl WidthRep for PortParam {}
