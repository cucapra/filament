use super::{
    Binding, Constraint, Id, InterfaceDef, OrderConstraint, PortDef, Range,
    TimeRep, WidthRep, WithTime,
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
    T: TimeRep,
{
    pub event: Id,
    pub delay: T::SubRep,
    pub default: Option<T>,
    pos: GPosIdx,
}

impl<T: TimeRep> WithTime<T> for EventBind<T> {
    fn events(&self) -> Vec<Id> {
        todo!("events for EventBind")
    }

    fn resolve_event(&self, bindings: &Binding<T>) -> Self {
        Self {
            delay: self.delay.resolve_event(bindings),
            default: self.default.as_ref().map(|d| d.resolve_event(bindings)),
            ..self.clone()
        }
    }

    fn resolve_offset(
        &self,
        bindings: &Binding<<T as TimeRep>::Offset>,
    ) -> Self {
        Self {
            delay: self.delay.resolve_offset(bindings),
            default: self.default.as_ref().map(|d| d.resolve_offset(bindings)),
            ..self.clone()
        }
    }
}

impl<T> WithPos for EventBind<T>
where
    T: TimeRep,
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
    T: TimeRep,
{
    pub fn new(event: Id, delay: T::SubRep, default: Option<T>) -> Self {
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
    T: TimeRep,
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
    Time: TimeRep,
    Width: WidthRep,
{
    /// Name of the component
    pub name: Id,
    /// Parameters for the Signature
    pub params: Vec<Id>,
    /// Unannotated ports that are threaded through by the backend
    pub unannotated_ports: Vec<(Id, u64)>,
    /// Mapping from name of signals to the abstract variable they provide
    /// evidence for.
    pub interface_signals: Vec<InterfaceDef>,
    /// Names of abstract variables bound by the component
    pub events: Vec<EventBind<Time>>,
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
    W: WidthRep,
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
    /// Get a port using its name
    pub fn get_port(&self, port: &Id) -> &PortDef<T, W> {
        self.ports
            .iter()
            .find(|p| p.name == *port)
            .unwrap_or_else(|| {
                panic!("Port {} not found in signature:\n{}", port, self.name)
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

    /// Constraints for well formed
    fn constraints(&self) -> impl Iterator<Item = Constraint<T>> + '_ {
        self.inputs()
            .chain(self.outputs())
            .flat_map(|mpd| mpd.liveness.well_formed())
    }

    /// Construct an event binding from this Signature's events and the given
    /// arguments.
    pub fn event_binding(&self, args: &[T]) -> Binding<T> {
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
                let bind =
                    eb.default.as_ref().unwrap().resolve_event(&partial_map);
                (eb.event.clone(), bind)
            })
            .collect();

        partial_map.extend(remaining);
        partial_map
    }

    /// Construct a parameter binding from this Signature's parameters and the
    pub fn param_binding<W0: WidthRep>(&self, args: &[W0]) -> Binding<W0> {
        debug_assert!(
            self.params.len() == args.len(),
            "Insuffient params for signature, required {} got {}",
            self.params.len(),
            args.len(),
        );

        Binding::new(self.params.iter().cloned().zip(args.iter().cloned()))
    }
}

impl<T, W> Signature<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    pub fn map<W0, F>(self, f: F) -> Signature<T, W0>
    where
        W0: Clone,
        F: Fn(W) -> W0,
        W0: WidthRep,
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

impl<T: TimeRep, W: WidthRep> Signature<T, W> {
    /// Constraints generated to ensure that a signature is well-formed.
    /// 1. Ensure that all the intervals are well formed
    /// 2. Ensure for each interval that mentions event `E` in its start time, the @interface
    ///    signal for `E` pulses less often than the length of the interval itself.
    pub fn well_formed(
        &self,
    ) -> FilamentResult<impl Iterator<Item = Constraint<T>> + '_> {
        let mut evs: HashMap<Id, Vec<_>> = HashMap::new();

        // Compute mapping from events to intervals to mention the event in their start time.
        // In the same way use of `E` in an invoke describes how often the invoke might trigger,
        // the start time of the signal describes when the signal is triggered.
        // We do not consider the end time because that only effects the length of the signal.
        for port in self.inputs().chain(self.outputs()) {
            let delay = port.liveness.len();
            let ev = &port.liveness.start.event();
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
                    Constraint::sub(OrderConstraint::gte(
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

impl<T: TimeRep, W: WidthRep> Signature<T, W> {
    /// Resolve a port definition using the given binding.
    fn resolve_port<WO: WidthRep>(
        &self,
        pd: &PortDef<T, W>,
        binding: &Binding<WO>,
    ) -> FilamentResult<PortDef<T, WO>> {
        if let Some(p) = pd.bitwidth.resolve(binding) {
            Ok(PortDef::new(pd.name.clone(), pd.liveness.clone(), p)
                .set_span(pd.copy_span()))
        } else {
            Err(Error::malformed(format!(
            "No binding for parameter {}. Port `{}.{}` is parameterized by `{}`",
            pd.bitwidth, self.name, pd.name, pd.bitwidth
        )))
        }
    }

    pub fn resolve<WO: WidthRep>(
        &self,
        args: &[WO],
    ) -> FilamentResult<Signature<T, WO>> {
        if args.len() != self.params.len() {
            return Err(Error::malformed(format!(
                "Cannot resolve signature. Expected {} arguments, provided {}",
                self.params.len(),
                args.len(),
            )));
        }

        let binding: Binding<WO> = self.param_binding(args);

        let resolved = Signature {
            params: vec![],
            ports: self
                .ports
                .clone()
                .iter()
                .map(|pd| self.resolve_port(pd, &binding))
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

impl<T, W> WithTime<T> for Signature<T, W>
where
    T: TimeRep,
    W: WidthRep,
{
    fn events(&self) -> Vec<Id> {
        todo!("events for signature")
    }

    fn resolve_event(&self, bindings: &Binding<T>) -> Self {
        Self {
            ports: self
                .ports
                .iter()
                .map(|pd| pd.resolve_event(bindings))
                .collect(),
            constraints: self
                .constraints
                .iter()
                .map(|c| c.resolve_event(bindings))
                .collect(),
            events: self
                .events
                .iter()
                .map(|eb| eb.resolve_event(bindings))
                .collect(),
            ..(self.clone())
        }
    }

    fn resolve_offset(
        &self,
        bindings: &Binding<<T as TimeRep>::Offset>,
    ) -> Self {
        Self {
            ports: self
                .ports
                .iter()
                .map(|pd| pd.resolve_offset(bindings))
                .collect(),
            constraints: self
                .constraints
                .iter()
                .map(|c| c.resolve_offset(bindings))
                .collect(),
            events: self
                .events
                .iter()
                .map(|eb| eb.resolve_offset(bindings))
                .collect(),
            ..(self.clone())
        }
    }
}

impl<T, W> Display for Signature<T, W>
where
    W: WidthRep,
    T: TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "comp {}{}<{}>({}) -> ({})",
            self.name,
            if self.params.is_empty() {
                "".to_string()
            } else {
                format!(
                    "[{}]",
                    self.params.iter().map(|p| p.to_string()).join(", ")
                )
            },
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
impl<W, T> std::fmt::Debug for Signature<T, W>
where
    W:,
    W: WidthRep,
    T: TimeRep,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
