mod checker;
mod context;
mod fact;

pub use context::{Context, Instance};

pub use checker::check;
pub use fact::{Fact, FactType};
