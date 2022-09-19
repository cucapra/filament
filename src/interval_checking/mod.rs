mod checker;
mod context;
mod guard_availability;
mod solver;

/// Constant used to represent this component
pub(super) const THIS: &str = "_this";

pub(super) use context::{ConcreteInvoke, Context};
pub(super) use guard_availability::total_interval;

pub use checker::check;
pub use solver::{FilSolver, SExp, ShareConstraints};
