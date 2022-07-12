use itertools::Itertools;

use super::{Command, Constraint, Id, Interval, Invoke, Range, TimeRep};
use crate::errors::{Error, FilamentResult, WithPos};
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

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

impl<T> PortDef<T>
where
    T: Clone + TimeRep,
{
    pub fn new(name: Id, liveness: Interval<T>, bitwidth: u64) -> Self {
        Self {
            name,
            liveness,
            bitwidth,
        }
    }
}
impl<T> Display for PortDef<T>
where
    T: Display + Clone + TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.liveness, self.name, self.bitwidth)
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
    delay: u64,
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

    pub fn delay(&self) -> u64 {
        self.delay
    }
}

/// The signature of a component definition
#[derive(Clone)]
pub struct Signature<T>
where
    T: Clone + TimeRep,
{
    /// Name of the component
    pub name: Id,

    /// Names of abstract variables bound by the component
    pub abstract_vars: Vec<Id>,

    /// Unannotated ports that are thread through by the backend
    pub unannotated_ports: Vec<(Id, u64)>,

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
            inputs: self.outputs.clone(),
            outputs: self.inputs.clone(),
            ..self.clone()
        }
    }

    /// Returns a port associated with the signature
    pub fn get_liveness<const IS_INPUT: bool>(
        &self,
        port: &Id,
    ) -> FilamentResult<Interval<T>> {
        let mut iter = if IS_INPUT {
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
                        Some(id.liveness.clone())
                    } else {
                        None
                    }
                })
            });

        maybe_pd.ok_or_else(|| {
            let kind = if IS_INPUT {
                "input port"
            } else {
                "output port"
            };
            Error::undefined(port.clone(), kind.to_string())
        })
    }

    // Return names of abstract variables that do not have a corresponding interface port defined
    // for them
    pub fn missing_interface_ports(&self) -> Vec<Id> {
        let defined_interfaces = self
            .interface_signals
            .iter()
            .map(|id| &id.event)
            .cloned()
            .collect();
        let all_events =
            self.abstract_vars.iter().cloned().collect::<HashSet<_>>();
        all_events
            .difference(&defined_interfaces)
            .cloned()
            .collect_vec()
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
            .flat_map(|mpd| mpd.liveness.well_formed())
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
            "component {}<{}>({}) -> ({})",
            self.name,
            self.abstract_vars
                .iter()
                .map(|id| id.to_string())
                .join(", "),
            self.unannotated_ports
                .iter()
                .map(|(n, bw)| format!("{n}: {bw}"))
                .chain(self.interface_signals.iter().map(|pd| format!("{pd}")))
                .chain(self.inputs.iter().map(|pd| format!("{pd}")))
                .join(", "),
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
    pub sig: Signature<T>,

    /// Model for this component
    pub body: Vec<Command<T>>,
}

impl<T> Component<T>
where
    T: Clone + TimeRep,
{
    pub fn new(
        sig: Signature<T>,
        body: Vec<Command<T>>,
    ) -> FilamentResult<Self> {
        let is_low = !sig.interface_signals.is_empty();

        if is_low {
            // All events should have corresponding interface signals
            let defined_interfaces = sig
                .interface_signals
                .iter()
                .map(|id| &id.event)
                .cloned()
                .collect();
            let all_events =
                sig.abstract_vars.iter().cloned().collect::<HashSet<_>>();
            if let Some(ev) = all_events.difference(&defined_interfaces).next()
            {
                return Err(Error::malformed("Low-level component does not have define interface port for event `{ev}`"));
            }

            // There should be no high-level invokes
            for con in body {
                match con {
                    Command::Invoke(inv @ Invoke { ports, .. }) => {
                        if ports.is_some() {
                            return Err(Error::malformed(
                                "Low-level component uses high-level invoke",
                            )
                            .with_pos(inv.copy_span()));
                        }
                    }
                    _ => (),
                }
            }
        } else {
            // There should be no FSM constructs or low-level invokes
            for con in body {
                match con {
                    Command::Invoke(inv @ Invoke { ports, .. }) => {
                        if ports.is_none() {
                            return Err(Error::malformed(
                                "High-level component uses low-level invoke",
                            )
                            .with_pos(inv.copy_span()));
                        }
                    }
                    Command::Fsm(fsm) => {
                        return Err(Error::malformed(
                            "High-level component uses FSM",
                        )
                        .with_pos(fsm.copy_span()))
                    }
                    _ => (),
                }
            }
        }

        Ok(Self { sig, body })
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

impl<T> Namespace<T>
where
    T: TimeRep + Clone,
{
    /// External signatures associated with the namespace
    pub fn signatures(&self) -> HashMap<Id, &Signature<T>> {
        self.externs
            .iter()
            .flat_map(|(_, comps)| comps.iter().map(|s| (s.name.clone(), s)))
            .collect()
    }
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
