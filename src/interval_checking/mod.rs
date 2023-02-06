mod checker;
mod context;
mod solver;

/// Constant used to represent this component
pub(super) const THIS: &str = "_this";

pub(super) use context::{ConcreteInvoke, Context};

pub use checker::check;
pub use solver::{FilSolver, SExp, ShareConstraints};
