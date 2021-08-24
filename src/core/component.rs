use super::{Control, Id, Interval};

pub struct PortDef {
    /// Name of the port
    pub name: Id,

    /// Liveness condition for the Port
    pub liveness: Interval,

    /// Bitwidth of the port
    pub bitwidth: u64,
}
impl std::fmt::Debug for PortDef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.liveness.fmt(f)?;
        write!(f, " {}: {}", self.name, self.bitwidth)
    }
}

pub struct Cell {
    /// Name of the instance.
    pub name: Id,
    /// Name of the component
    pub component: Id,
}

/// The signature of a component definition
#[derive(Debug)]
pub struct Signature {
    /// Name of the component
    pub name: Id,

    /// Names of abstract variables bound by the component
    pub abstract_vars: Vec<Id>,

    /// Input ports
    pub inputs: Vec<PortDef>,

    /// Output ports
    pub outputs: Vec<PortDef>,
}

/// A component in Filament
pub struct Component {
    // Signature of this component
    pub sig: Signature,

    /// Names of sub-circuits used in constructing this component
    pub cells: Vec<Cell>,

    /// Model for this component
    pub body: Vec<Control>,
}

pub struct Namespace {
    /// External definitions without a body.
    pub signatures: Vec<Signature>,

    /// Components defined in this file
    pub components: Vec<Component>,
}
