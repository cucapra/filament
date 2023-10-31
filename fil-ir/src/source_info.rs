use super::{utils::SparseInfoMap, Event, Param, Port};
use fil_ast as ast;

#[derive(Clone)]
/// Externally facing interface name information for components.
pub struct InterfaceSrc {
    /// The name of the component
    pub name: ast::Id,
    /// Mapping from port indices to their source visible names
    pub ports: SparseInfoMap<Port, ast::Id>,
    /// Mapping from parameter indices to their source visible names
    pub params: SparseInfoMap<Param, ast::Id>,
    /// Mapping from event indices to their source visible names
    pub events: SparseInfoMap<Event, ast::Id>,
    /// Mapping from event indices the source port that implements their interface
    pub interface_ports: SparseInfoMap<Event, ast::Id>,
    /// The external tool that generates this module during compilation
    pub gen_tool: Option<String>,
}

impl InterfaceSrc {
    pub fn new(name: ast::Id, gen_tool: Option<String>) -> Self {
        Self {
            name,
            ports: SparseInfoMap::default(),
            params: SparseInfoMap::default(),
            interface_ports: SparseInfoMap::default(),
            events: SparseInfoMap::default(),
            gen_tool,
        }
    }
}
