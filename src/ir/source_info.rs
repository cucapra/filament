use super::{EventIdx, ParamIdx, PortIdx};
use crate::ast;
use std::collections::HashMap;

#[derive(Clone)]
/// Externally facing interface name information for components.
pub struct InterfaceSrc {
    /// The name of the component
    pub name: ast::Id,
    /// Mapping from port indices to their source visible names
    pub ports: HashMap<PortIdx, ast::Id>,
    /// Mapping from parameter indices to their source visible names
    pub params: HashMap<ParamIdx, ast::Id>,
    /// Mapping from event indices to their source visible names
    pub events: HashMap<EventIdx, ast::Id>,
    /// Mapping from event indices the source port that implements their interface
    pub interface_ports: HashMap<EventIdx, ast::Id>,
}

impl InterfaceSrc {
    pub fn new(name: ast::Id) -> Self {
        Self {
            name,
            ports: HashMap::new(),
            params: HashMap::new(),
            interface_ports: HashMap::new(),
            events: HashMap::new(),
        }
    }
}
