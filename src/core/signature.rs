use super::{
    Binding, Constraint, ConstraintBase, Id, InterfaceDef, Interval, PortDef,
    PortParam, Time, TimeRep,
};
use crate::errors::{Error, FilamentResult, WithPos};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

#[derive(Clone)]
/// An event variable bound in the signature
pub struct EventBind<T>
where
    T: Clone + TimeRep,
{
    pub event: Id,
    pub default: Option<T>,
}

impl<T> EventBind<T>
where
    T: Clone + TimeRep,
{
    pub fn new(event: Id) -> Self {
        Self {
            event,
            default: None,
        }
    }
}
impl<T> Display for EventBind<T>
where
    T: TimeRep + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(default) = &self.default {
            write!(f, "?{}={}", self.event, default)
        } else {
            write!(f, "{}", self.event)
        }
    }
}

/// The signature of a component definition
#[derive(Clone)]
pub struct Signature<T, W>
where
    T: Clone + TimeRep,
    W: Clone,
{
    /// Name of the component
    pub name: Id,

    /// Parameters for the Signature
    pub params: Vec<Id>,

    /// Names of abstract variables bound by the component
    pub abstract_vars: Vec<EventBind<T>>,

    /// Unannotated ports that are thread through by the backend
    pub unannotated_ports: Vec<(Id, u64)>,

    /// Mapping from name of signals to the abstract variable they provide
    /// evidence for.
    pub interface_signals: Vec<InterfaceDef<T>>,

    /// Input ports
    pub inputs: Vec<PortDef<T, W>>,

    /// Output ports
    pub outputs: Vec<PortDef<T, W>>,

    /// Constraints on the abstract variables in the signature
    pub constraints: Vec<Constraint<T>>,
}

impl<T, W> Signature<T, W>
where
    T: TimeRep,
    W: Clone,
{
    /// Events bound by the signature
    pub fn events(&self) -> impl Iterator<Item = &Id> {
        self.abstract_vars.iter().map(|eb| &eb.event)
    }

    pub fn abstract_vars_with_defaults(
        &self,
    ) -> impl Iterator<Item = &EventBind<T>> {
        self.abstract_vars.iter()
    }

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

        maybe_pd.ok_or_else(|| panic!("Unknown port: {}", port))
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
    pub fn binding(&self, args: &[T]) -> Binding<T> {
        debug_assert!(
            self.abstract_vars
                .iter()
                .take_while(|ev| ev.default.is_none())
                .count()
                <= args.len(),
            "Insuffient events for signature, required at least {} got {}",
            self.abstract_vars
                .iter()
                .take_while(|ev| ev.default.is_none())
                .count(),
            args.len(),
        );

        let mut partial_map = Binding::new(
            self.abstract_vars
                .iter()
                .map(|eb| &eb.event)
                .cloned()
                .zip(args.iter().cloned())
                .collect(),
        );
        // Skip the events that have been bound
        let remaining = self
            .abstract_vars
            .iter()
            .skip(args.len())
            .map(|eb| {
                let bind = eb.default.as_ref().unwrap().resolve(&partial_map);
                (eb.event.clone(), bind)
            })
            .collect();

        partial_map.extend(remaining);
        partial_map
    }
}

impl<W: Clone> Signature<Time<u64>, W> {
    /// Constraints generated to ensure that a signature is well-formed.
    /// 1. Ensure that all the intervals are well formed
    /// 2. Ensure for each interval that mentions event `E` in its start time, the @interface
    ///    signal for `E` pulses less often than the length of the interval itself.
    pub fn well_formed(
        &self,
    ) -> FilamentResult<impl Iterator<Item = Constraint<Time<u64>>> + '_> {
        let mut evs: HashMap<Id, Vec<_>> = HashMap::new();

        // Compute mapping from events to intervals to mention the event in their start time.
        // In the same way use of `E` in an invoke describes how often the invoke might trigger,
        // the start time of the signal describes when the signal is triggered.
        // We do not consider the end time because that only effects the length of the signal.

        for port in self.inputs.iter().chain(self.outputs.iter()) {
            let delay = port.liveness.within.len();
            let ev = &port.liveness.within.start.event;
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

impl<T: TimeRep> Signature<T, PortParam> {
    pub fn resolve(&self, args: &[u64]) -> FilamentResult<Signature<T, u64>> {
        if args.len() != self.params.len() {
            return Err(Error::malformed(format!(
                "Cannot resolve signature. Expected {} arguments, provided {}",
                self.params.len(),
                args.len(),
            )));
        }

        let binding: HashMap<Id, u64> = self
            .params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();
        let resolve_port =
            |pd: &PortDef<T, PortParam>| -> FilamentResult<PortDef<T, u64>> {
                match &pd.bitwidth {
                    PortParam::Const(c) => Ok(PortDef::new(
                        pd.name.clone(),
                        pd.liveness.clone(),
                        *c,
                    )
                    .set_span(pd.copy_span())),
                    PortParam::Var(param) => {
                        if let Some(&c) = binding.get(param) {
                            Ok(PortDef::new(
                                pd.name.clone(),
                                pd.liveness.clone(),
                                c,
                            )
                            .set_span(pd.copy_span()))
                        } else {
                            Err(Error::malformed(format!(
                                "No binding for parameter {}. Port `{}.{}` is parameterized by `{}`",
                                param, self.name, pd.name, param
                            )))
                        }
                    }
                }
            };

        let resolved = Signature {
            name: self.name.clone(),
            abstract_vars: self.abstract_vars.clone(),
            params: vec![],
            unannotated_ports: self.unannotated_ports.clone(),
            interface_signals: self.interface_signals.clone(),
            inputs: self
                .inputs
                .iter()
                .map(resolve_port)
                .collect::<FilamentResult<_>>()?,
            outputs: self
                .outputs
                .iter()
                .map(resolve_port)
                .collect::<FilamentResult<_>>()?,
            constraints: self.constraints.clone(),
        };

        Ok(resolved)
    }
}

impl<T, W> Display for Signature<T, W>
where
    T: Display + Clone + TimeRep,
    W: Clone + Display,
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
impl<T, W> std::fmt::Debug for Signature<T, W>
where
    T: Display + Clone + TimeRep,
    W: Display + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
