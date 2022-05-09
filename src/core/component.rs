use std::rc::Rc;

use super::{interval, Command, Id, Interval};

#[derive(Clone)]
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

    /// Constraints on the abstract variables in the signature
    pub constraints: Vec<interval::Constraint>,
}

impl Signature {
    // Generate a new signature that has been reversed: inputs are outputs
    // with outputs.
    pub fn reversed(&self) -> Self {
        Self {
            name: self.name.clone(),
            abstract_vars: self.abstract_vars.clone(),
            inputs: self.outputs.clone(),
            outputs: self.inputs.clone(),
            constraints: self.constraints.clone(),
        }
    }
}

/// A component in Filament
pub struct Component {
    // Signature of this component
    pub sig: Rc<Signature>,

    /// Model for this component
    pub body: Vec<Command>,
}

pub struct Namespace {
    /// Imported files
    pub imports: Vec<String>,

    /// External definitions without a body.
    pub signatures: Vec<Signature>,

    /// Components defined in this file
    pub components: Vec<Component>,
}
