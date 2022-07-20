use super::{
    Binding, Constraint, ConstraintBase, FsmIdxs, Id, InterfaceDef, Interval,
    PortDef, TimeRep,
};
use crate::errors::{Error, FilamentResult, WithPos};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

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
    pub constraints: Vec<Constraint<T>>,
}

impl<T> Signature<T>
where
    T: TimeRep,
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

    /// Return the interface associated with an event defined in the signature.
    pub fn get_interface(&self, event: &Id) -> Option<&InterfaceDef<T>> {
        self.interface_signals.iter().find(|id| id.event == event)
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

    /// Constraints for well formed under a binding
    fn constraints(&self) -> impl Iterator<Item = Constraint<T>> + '_ {
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

    /// Construct a binding from this Signature
    pub fn binding<'a>(&self, args: &'a [T]) -> FilamentResult<Binding<'a, T>> {
        if self.abstract_vars.len() != args.len() {
            return Err(Error::malformed(format!(
                "Cannot construct binding. Expected {} arguments, provided {}",
                self.abstract_vars.len(),
                args.len(),
            )));
        }

        Ok(Binding::new(
            self.abstract_vars
                .iter()
                .cloned()
                .zip(args.iter())
                .collect(),
        ))
    }
}

impl Signature<FsmIdxs> {
    /// Constraints generated to ensure that a signature is well-formed.
    /// 1. Ensure that all the intervals are well formed
    /// 2. Ensure for each interval that mentions event `E` in its start time, the @interface
    ///    signal for `E` pulses less often than the length of the interval itself.
    pub fn well_formed(
        &self,
    ) -> FilamentResult<impl Iterator<Item = Constraint<FsmIdxs>> + '_> {
        let mut evs: HashMap<Id, Vec<_>> = HashMap::new();

        // Compute mapping from events to intervals to mention the event in their start time.
        // In the same way use of `E` in an invoke describes how often the invoke might trigger,
        // the start time of the signal describes when the signal is triggered.
        // We do not consider the end time because that only effects the length of the signal.

        for port in self.inputs.iter().chain(self.outputs.iter()) {
            let delay = port.liveness.within.len();
            for (ev, _) in port.liveness.within.start.events() {
                // Make sure @interface for event exists
                if self.get_interface(ev).is_none() {
                    return Err(Error::malformed(format!(
                        "Missing @interface port for {ev}"
                    ))
                    .add_note(
                        format!("Port mentions `{ev}` event in its start time"),
                        port.liveness.within.copy_span(),
                    ));
                }
                evs.entry(ev.clone())
                    .or_default()
                    .push((delay.clone(), port.copy_span()))
            }
        }

        Ok(evs
            .into_iter()
            .flat_map(|(ev, lens)| {
                let id = self.get_interface(&ev).unwrap();
                let len = id.delay();
                lens.into_iter().map(move |(port_len, port_pos)| {
                    Constraint::from(ConstraintBase::gte(
                        len.clone(),
                        port_len.clone(),
                    ))
                    .add_note("Invalid interface", id.copy_span())
                    .add_note(
                        format!("Signal lasts for {}", port_len),
                        port_pos,
                    )
                    .add_note(
                        format!("Interface signal lasts for {}", len),
                        id.copy_span(),
                    )
                })
            })
            .chain(self.constraints()))
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
