use crate::core;
pub trait Pass
{
    fn transform(ns: core::Namespace) -> core::Namespace;
}
