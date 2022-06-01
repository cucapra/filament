use std::rc::Rc;

use super::{interval, Command, Id, Interval, TimeRep};

#[derive(Clone)]
pub struct PortDef<T>
where
    T: Clone + TimeRep,
{
    /// Name of the port
    pub name: Id,

    /// Liveness condition for the Port
    pub liveness: Interval<T>,

    /// Bitwidth of the port
    pub bitwidth: u64,
}
impl<T> std::fmt::Debug for PortDef<T>
where
    T: std::fmt::Debug + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.liveness.fmt(f)?;
        write!(f, " {}: {}", self.name, self.bitwidth)
    }
}

/// The signature of a component definition
#[derive(Debug)]
pub struct Signature<T>
where
    T: Clone + TimeRep,
{
    /// Name of the component
    pub name: Id,

    /// Names of abstract variables bound by the component
    pub abstract_vars: Vec<Id>,

    /// Input ports
    pub inputs: Vec<PortDef<T>>,

    /// Output ports
    pub outputs: Vec<PortDef<T>>,

    /// Constraints on the abstract variables in the signature
    pub constraints: Vec<interval::Constraint<T>>,
}

impl<T> Signature<T>
where
    T: Clone + TimeRep,
{
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
pub struct Component<T>
where
    T: Clone + TimeRep,
{
    // Signature of this component
    pub sig: Rc<Signature<T>>,

    /// Model for this component
    pub body: Vec<Command<T>>,
}

impl<T> Component<T>
where
    T: Clone + TimeRep,
{
    pub fn new(sig: Signature<T>, body: Vec<Command<T>>) -> Self {
        Self {
            sig: Rc::new(sig),
            body,
        }
    }
}

pub struct Namespace<T>
where
    T: Clone + TimeRep,
{
    /// Imported files
    pub imports: Vec<String>,

    /// External definitions without a body.
    pub signatures: Vec<Signature<T>>,

    /// Components defined in this file
    pub components: Vec<Component<T>>,
}
