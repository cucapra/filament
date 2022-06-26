use itertools::Itertools;

use super::{Command, Constraint, Id, Interval, Range, TimeRep};
use crate::errors::{Error, FilamentResult};
use std::{fmt::Display, rc::Rc};

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
impl<T> Display for PortDef<T>
where
    T: Display + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(liv) = &self.liveness {
            write!(f, "{} ", liv)?;
        }
        write!(f, "{}: {}", self.name, self.bitwidth)
    }
}

#[derive(Clone)]
pub struct InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    /// Name of the port
    pub name: Id,
    // Event that this port is an evidence of
    pub event: Id,
    // Delay required for this signal
    pub delay: u64,
    // Liveness of the interface signal
    liveness: Interval<T>,
}
impl<T> Display for InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "@interface<{}, {}> {}: 1",
            self.event, self.delay, self.name
        )
    }
}

impl<T> InterfaceDef<T>
where
    T: TimeRep + Clone,
{
    pub fn new(name: Id, event: Id, delay: u64) -> Self
    where
        T: TimeRep + Clone,
    {
        let start = T::unit(event.clone(), 0);
        let liveness = Interval::from(Range::new(
            start.clone(),
            start.clone().increment(delay),
        ))
        .with_exact(Range::new(start.clone(), start.increment(1)));
        Self {
            name,
            event,
            delay,
            liveness,
        }
    }
}

/// The signature of a component definition
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
    pub interface_signals: Vec<InterfaceDef<T>>,

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
    pub fn get_liveness(
        &self,
        port: &Id,
        is_input: bool,
    ) -> FilamentResult<Option<Interval<T>>> {
        let mut iter = if is_input {
            self.inputs.iter()
        } else {
            self.outputs.iter()
        };

        // XXX(rachit): Always searching interface ports regardless of input or output
        let maybe_pd = iter
            .find_map(|pd| {
                if pd.name == port {
                    Some(pd.liveness.clone())
                } else {
                    None
                }
            })
            .or_else(|| {
                self.interface_signals.iter().find_map(|id| {
                    if id.name == port {
                        Some(Some(id.liveness.clone()))
                    } else {
                        None
                    }
                })
            });

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
            .flat_map(|mpd| {
                mpd.liveness
                    .as_ref()
                    .map(|pd| pd.well_formed())
                    .unwrap_or_default()
            })
            .chain(
                self.interface_signals
                    .iter()
                    .flat_map(|id| id.liveness.well_formed()),
            )
    }
}
impl<T> Display for Signature<T>
where
    T: Display + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "component {}<{}>(",
            self.name,
            self.abstract_vars
                .iter()
                .map(|id| id.to_string())
                .join(", "),
        )?;
        if !self.interface_signals.is_empty() {
            write!(
                f,
                "{}",
                self.interface_signals
                    .iter()
                    .map(|pd| format!("{pd}"))
                    .join(", "),
            )?;
            if !self.inputs.is_empty() {
                write!(f, ", ")?;
            }
        }
        write!(
            f,
            "{}) -> ({})",
            self.inputs.iter().map(|pd| format!("{pd}")).join(", "),
            self.outputs.iter().map(|pd| format!("{pd}")).join(", "),
        )?;
        if !self.constraints.is_empty() {
            write!(
                f,
                " where {}",
                self.constraints
                    .iter()
                    .map(|cons| format!("{cons}"))
                    .join(", "),
            )?;
        }
        Ok(())
    }
}
impl<T> std::fmt::Debug for Signature<T>
where
    T: Display + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
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
impl<T> Display for Component<T>
where
    T: Clone + TimeRep + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{} {{", self.sig)?;
        for com in &self.body {
            writeln!(f, "  {};", com)?;
        }
        writeln!(f, "}}")
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

impl<T> Display for Namespace<T>
where
    T: Clone + TimeRep + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for imp in &self.imports {
            writeln!(f, "import \"{}\";", imp)?;
        }
        for (path, sigs) in &self.externs {
            writeln!(f, "extern \"{}\" {{", path)?;
            for sig in sigs {
                writeln!(f, "  {};", sig)?;
            }
            writeln!(f, "}}")?;
        }
        for comp in &self.components {
            writeln!(f, "{}", comp)?;
        }
        Ok(())
    }
}
