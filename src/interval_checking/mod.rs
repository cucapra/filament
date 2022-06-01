mod checker;
mod context;
mod fact;
mod guard_availability;
mod solver;

/// Representation of time in this module
pub type TimeRep = crate::core::FsmIdxs;

/// Constant used to represent this component
pub(super) const THIS: &str = "_this";

pub(super) use context::{ConcreteInvoke, Context};
pub(super) use fact::Fact;
pub(super) use guard_availability::total_interval;

pub use checker::check;
pub use solver::{prove, SExp};
