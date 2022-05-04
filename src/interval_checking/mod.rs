mod checker;
mod context;
mod fact;
mod solver;

pub(super) use context::{ConcreteInvoke, Context};
pub(super) use fact::Fact;
pub use solver::{prove, SExp};

pub use checker::check;
