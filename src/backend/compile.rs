use std::collections::HashMap;

use crate::{core, errors::FilamentResult};

/// Representation of an FSM
pub struct FSM {}

/// Compilation context
#[derive(Default)]
struct Context {
    /// Mapping from an abstract variable to the FSM it represents
    abs_map: HashMap<core::Id, FSM>,
}

fn compile() -> FilamentResult<()> {
    todo!()
}
