use super::{Command, Constraint, Id, Interval, TimeRep};
use crate::errors::{Error, FilamentResult};
use std::rc::Rc;

#[derive(Clone)]
pub struct PortDef<T>
where
    T: Clone + TimeRep,
{
    /// Name of the port
    pub name: Id,

    /// Liveness condition for the Port
    pub liveness: Option<Interval<T>>,

    /// Bitwidth of the port
    pub bitwidth: u64,
}

impl<T> PortDef<T>
where
    T: Clone + TimeRep,
{
    pub fn new(
        name: Id,
        liveness: Option<Interval<T>>,
        bitwidth: u64,
    ) -> FilamentResult<Self> {
        Ok(Self {
            name,
            liveness,
            bitwidth,
        })
    }
}
impl<T> std::fmt::Debug for PortDef<T>
where
    T: std::fmt::Debug + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {}: {}", self.liveness, self.name, self.bitwidth)
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

    /// Mapping from name of signals to the abstract variable they provide
    /// evidence for.
    pub interface_signals: Vec<PortDef<T>>,

    /// Input ports
    pub inputs: Vec<PortDef<T>>,

    /// Output ports
    pub outputs: Vec<PortDef<T>>,

    /// Constraints on the abstract variables in the signature
    pub constraints: Vec<super::Constraint<T>>,
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
            interface_signals: self.interface_signals.clone(),
            inputs: self.outputs.clone(),
            outputs: self.inputs.clone(),
            constraints: self.constraints.clone(),
        }
    }

    /// Returns a port associated with the signature
    pub fn get_port(
        &self,
        port: &Id,
        is_input: bool,
    ) -> FilamentResult<&PortDef<T>> {
        // XXX(rachit): Always searching interface ports regardless of input or output
        let maybe_pd = if is_input {
            self.inputs
                .iter()
                .chain(self.interface_signals.iter())
                .find(|pd| pd.name == port)
        } else {
            self.outputs
                .iter()
                .chain(self.interface_signals.iter())
                .find(|pd| pd.name == port)
        };
        maybe_pd.ok_or_else(|| {
            let kind = if is_input {
                "input port"
            } else {
                "output port"
            };
            Error::undefined(port.clone(), kind.to_string())
        })
    }
}
impl<T> Signature<T>
where
    T: Clone + TimeRep + PartialEq + PartialOrd,
{
    /// Constraints for well formed under a binding
    pub fn well_formed(&self) -> impl Iterator<Item = Constraint<T>> + '_ {
        self.inputs
            .iter()
            .chain(self.outputs.iter())
            .chain(self.interface_signals.iter())
            .flat_map(|mpd| {
                mpd.liveness
                    .as_ref()
                    .map(|pd| pd.well_formed())
                    .unwrap_or_default()
            })
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

    /// Define externals and their files
    pub externs: Vec<(String, Vec<Signature<T>>)>,

    /// Components defined in this file
    pub components: Vec<Component<T>>,
}
