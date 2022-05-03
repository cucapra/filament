mod checker;
mod context;
mod fact;

pub use context::{ConcreteInvoke, Context};

pub use checker::check;
pub use fact::{Fact, FactType};
