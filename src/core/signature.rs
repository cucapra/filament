use super::{
    Binding, Constraint, Id, InterfaceDef, OrderConstraint, PortDef, PortParam,
    Range, Time, TimeRep, TimeSub,
};
use crate::{
    errors::{Error, FilamentResult, WithPos},
    utils::GPosIdx,
};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

#[derive(Clone)]
/// An event variable bound in the signature
pub struct EventBind<T>
where
    T: Clone + TimeRep,
{
    pub event: Id,
    pub delay: TimeSub<T>,
    pub default: Option<T>,
    pos: GPosIdx,
}

impl<T> WithPos for EventBind<T>
where
    T: Clone + TimeRep,
{
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

impl<T> EventBind<T>
where
    T: Clone + TimeRep,
{
    pub fn new(event: Id, delay: TimeSub<T>, default: Option<T>) -> Self {
        Self {
            event,
            delay,
            default,
            pos: GPosIdx::UNKNOWN,
        }
    }
}
impl<T> Display for EventBind<T>
where
    T: TimeRep + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(default) = &self.default {
            write!(f, "?{}: {}={}", self.event, self.delay, default)
        } else {
            write!(f, "{}: {}", self.event, self.delay)
        }
    }
}

/// The signature of a component definition
#[derive(Clone)]
pub struct Signature<Time, Width>
where
    Time: Clone + TimeRep,
    Width: Clone,
{
    /// Name of the component
    pub name: Id,
    /// Parameters for the Signature
    pub params: Vec<Id>,
    /// Names of abstract variables bound by the component
    pub events: Vec<EventBind<Time>>,
    /// Unannotated ports that are threaded through by the backend
    pub unannotated_ports: Vec<(Id, u64)>,
    /// Mapping from name of signals to the abstract variable they provide
    /// evidence for.
    pub interface_signals: Vec<InterfaceDef>,
    /// Constraints on the abstract variables in the signature
    pub constraints: Vec<Constraint<Time>>,
    /// All the input/output ports.
    ports: Vec<PortDef<Time, Width>>,
    /// Index of the first output port in the ports vector
    outputs_idx: usize,
}

impl<T, W> Signature<T, W>
where
    T: TimeRep,
    W: Clone,
{
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Id,
        params: Vec<Id>,
        events: Vec<EventBind<T>>,
        unannotated_ports: Vec<(Id, u64)>,
        interface_signals: Vec<InterfaceDef>,
        mut inputs: Vec<PortDef<T, W>>,
        mut outputs: Vec<PortDef<T, W>>,
        constraints: Vec<Constraint<T>>,
    ) -> Self {
        let outputs_idx = inputs.len();
        inputs.append(&mut outputs);
        Self {
            name,
            params,
            events,
            unannotated_ports,
            interface_signals,
            ports: inputs,
            outputs_idx,
            constraints,
        }
    }

    /// Events bound by the signature
    pub fn events(&self) -> impl Iterator<Item = Id> + '_ {
        self.events.iter().map(|eb| &eb.event).cloned()
    }
    /// Inputs of this signature
    pub fn inputs(&self) -> impl Iterator<Item = &PortDef<T, W>> {
        self.ports[..self.outputs_idx].iter()
    }
    /// Outputs of this signature
    pub fn outputs(&self) -> impl Iterator<Item = &PortDef<T, W>> {
        self.ports[self.outputs_idx..].iter()
    }
    /// Iterator over all the ports of this signature
    pub fn ports(&self) -> impl Iterator<Item = &PortDef<T, W>> {
        self.ports.iter()
    }

    /// Find the delay associoated with an event
    pub fn get_event(&self, event: &Id) -> &EventBind<T> {
        self.events
            .iter()
            .find(|eb| eb.event == event)
            .unwrap_or_else(|| {
                panic!("Event {} not found in signature:\n{}", event, self.name)
            })
    }

    // Generate a new signature that has been reversed: inputs are outputs
    // with outputs.
    pub fn reversed(&self) -> Self {
        let mut ports = self.outputs().cloned().collect_vec();
        let outputs_idx = ports.len();
        ports.extend(self.inputs().cloned());
        Self {
            ports,
            outputs_idx,
            ..self.clone()
        }
    }

    /// Return the interface associated with an event defined in the signature.
    pub fn get_interface(&self, event: &Id) -> Option<&InterfaceDef> {
        self.interface_signals.iter().find(|id| id.event == event)
    }

    /// Returns a port associated with the signature
    pub fn get_liveness<const IS_INPUT: bool>(
        &self,
        port: &Id,
    ) -> FilamentResult<Range<T>> {
        let ports = if IS_INPUT {
            &self.ports[..self.outputs_idx]
        } else {
            &self.ports[self.outputs_idx..]
        };

        // XXX(rachit): Always searching interface ports regardless of input or output
        let maybe_pd = ports
            .iter()
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
                        // Interface signals are always active between [E, E+1]
                        Some(Range::new(
                            TimeRep::unit(id.event.clone(), 0),
                            TimeRep::unit(id.event.clone(), 1),
                        ))
                    } else {
                        None
                    }
                })
            });

        maybe_pd.ok_or_else(|| panic!("Unknown port: {}", port))
    }

    /// Iterate over all phantom events. A phantom event is an event that does not have a corresponding interface signal.
    pub fn phantom_events(&self) -> impl Iterator<Item = Id> + '_ {
        self.events()
            .filter(move |event| self.get_interface(event).is_none())
    }

    /// Constraints for well formed under a binding
    fn constraints(&self) -> impl Iterator<Item = Constraint<T>> + '_ {
        self.inputs()
            .chain(self.outputs())
            .flat_map(|mpd| mpd.liveness.well_formed())
    }

    /// Construct a binding from this Signature
    pub fn binding(&self, args: &[T]) -> Binding<T> {
        debug_assert!(
            self.events
                .iter()
                .take_while(|ev| ev.default.is_none())
                .count()
                <= args.len(),
            "Insuffient events for signature, required at least {} got {}",
            self.events
                .iter()
                .take_while(|ev| ev.default.is_none())
                .count(),
            args.len(),
        );

        let mut partial_map = Binding::new(
            self.events
                .iter()
                .map(|eb| &eb.event)
                .cloned()
                .zip(args.iter().cloned()),
        );
        // Skip the events that have been bound
        let remaining = self
            .events
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

impl<T: TimeRep, W: Clone> Signature<T, W> {
    pub fn map<W0, F>(self, f: F) -> Signature<T, W0>
    where
        W0: Clone,
        F: Fn(W) -> W0,
    {
        Signature {
            name: self.name,
            ports: self
                .ports
                .into_iter()
                .map(|pd| PortDef::new(pd.name, pd.liveness, f(pd.bitwidth)))
                .collect(),
            outputs_idx: self.outputs_idx,
            params: self.params,
            unannotated_ports: self.unannotated_ports,
            constraints: self.constraints,
            events: self.events,
            interface_signals: self.interface_signals,
        }
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
        for port in self.inputs().chain(self.outputs()) {
            let delay = port.liveness.len();
            let ev = &port.liveness.start.event;
            evs.entry(ev.clone())
                .or_default()
                .push((delay.clone(), port.copy_span()))
        }

        Ok(evs
            .into_iter()
            .flat_map(|(ev, lens)| {
                let event = self.get_event(&ev);
                lens.into_iter().map(move |(port_len, port_pos)| {
                    let len = event.delay.clone();
                    Constraint::from(OrderConstraint::gte(
                        len.clone(),
                        port_len.clone(),
                    ))
                    .add_note(
                        format!("Signal lasts for {} cycle(s)", port_len),
                        port_pos,
                    )
                    .add_note(
                        format!("Interface allows event to trigger every {} cycle(s)", len),
                        event.copy_span(),
                    )
                })
            })
            .chain(self.constraints()))
    }
}

impl<T: TimeRep> Signature<T, PortParam> {
    pub fn resolve(
        &self,
        args: &[PortParam],
    ) -> FilamentResult<Signature<T, PortParam>> {
        if args.len() != self.params.len() {
            return Err(Error::malformed(format!(
                "Cannot resolve signature. Expected {} arguments, provided {}",
                self.params.len(),
                args.len(),
            )));
        }

        let binding: HashMap<Id, PortParam> = self
            .params
            .iter()
            .cloned()
            .zip(args.iter().cloned())
            .collect();

        let resolve_port =
            |pd: &PortDef<T, PortParam>| -> FilamentResult<PortDef<T, PortParam>> {
                match &pd.bitwidth {
                    PortParam::Const(_) => Ok(pd.clone()),
                    PortParam::Var(param) => {
                        if let Some(c) = binding.get(param) {
                            Ok(PortDef::new(
                                pd.name.clone(),
                                pd.liveness.clone(),
                                c.clone(),
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
            params: vec![],
            ports: self
                .ports
                .clone()
                .iter()
                .map(resolve_port)
                .collect::<FilamentResult<_>>()?,
            // Clone everything else
            outputs_idx: self.outputs_idx,
            name: self.name.clone(),
            events: self.events.clone(),
            interface_signals: self.interface_signals.clone(),
            unannotated_ports: self.unannotated_ports.clone(),
            constraints: self.constraints.clone(),
        };

        Ok(resolved)
    }
}

impl<W> Display for Signature<Time<u64>, W>
where
    W: Clone + Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "comp {}<{}>({}) -> ({})",
            self.name,
            self.events.iter().map(|id| id.to_string()).join(", "),
            self.unannotated_ports
                .iter()
                .map(|(n, bw)| format!("{n}: {bw}"))
                .chain(self.interface_signals.iter().map(|pd| format!("{pd}")))
                .chain(self.inputs().map(|pd| format!("{pd}")))
                .join(", "),
            self.outputs().map(|pd| format!("{pd}")).join(", "),
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
impl<W> std::fmt::Debug for Signature<Time<u64>, W>
where
    W: Display + Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
