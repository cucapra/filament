use std::{fmt::Display, collections::HashMap};
use itertools::Itertools;
use crate::errors::{FilamentResult, Error, WithPos};
use super::{TimeRep, Id, InterfaceDef, PortDef, Constraint, Interval, FsmIdxs, ConstraintBase};

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

impl Signature<FsmIdxs> {
    /// Validate a signature instance.
    pub fn validate(&self) -> impl Iterator<Item = Constraint<FsmIdxs>> + '_ {
        // The interface is invalid if the interface signal is shorter than
        // an input signal's requirement.
        let mut max_evs: HashMap<_, _> = self
            .abstract_vars
            .iter()
            .map(|ev| (ev, (0, None)))
            .collect();

        for port in &self.inputs {
            port.liveness.events().into_iter().for_each(|ev| {
                ev.events().for_each(|(ev, st)| {
                    if max_evs[ev].0 < *st {
                        *max_evs.get_mut(ev).unwrap() = (*st, port.copy_span());
                    }
                })
            });
        }

        self.interface_signals.iter().map(move |id| {
            let (st, pos) = &max_evs[&id.event];
            let cons: Constraint<FsmIdxs> = ConstraintBase::lt(
                FsmIdxs::unit(id.event.clone(), *st),
                id.end.clone(),
            )
            .into();
            cons.add_note("Interface does not last long enough", id.copy_span())
                .add_note(
                    "Input signal's requirement lasts longer",
                    pos.clone(),
                )
        })
    }
}
