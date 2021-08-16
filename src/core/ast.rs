use super::{Id, Interval};

pub struct Port {
    /// Name of the port
    pub name: Id,

    /// Liveness condition for the Port
    pub liveness: Interval,

    /// Bitwidth of the port
    pub bitwidth: u64,
}

pub struct Body;

/// The signature of a component definition
pub struct Signature {
    /// Names of abstract variables bound by the component
    pub abstract_vars: Vec<Id>,

    /// Input ports
    pub inputs: Vec<Port>,

    /// Output ports
    pub outputs: Vec<Port>,
}

/// A component in Filament
pub struct Component {
    /// Name of the component
    pub name: Id,

    // Signature of this component
    pub sig: Signature,

    /// Names of sub-circuits used in constructing this component
    pub cells: Vec<Id>,

    /// Model for this component
    pub body: Body,
}

pub struct Namespace {
    /// Components defined in this file
    pub components: Vec<Component>,
}
