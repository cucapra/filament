use super::{
    Binding, ConcTime, Constraint, Id, InterfaceDef, OrderConstraint, PortDef,
    TimeSub, Width,
};
use crate::{errors::WithPos, utils::GPosIdx};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

#[derive(Clone)]
/// An event variable bound in the signature
pub struct EventBind {
    pub event: Id,
    pub delay: TimeSub,
    pub default: Option<ConcTime>,
    pos: GPosIdx,
}

impl EventBind {
    fn resolve_event(&self, bindings: &Binding<ConcTime>) -> Self {
        Self {
            delay: self.delay.resolve_event(bindings),
            default: self.default.as_ref().map(|d| d.resolve_event(bindings)),
            ..self.clone()
        }
    }

    fn resolve_offset(&self, bindings: &Binding<Width>) -> Self {
        Self {
            delay: self.delay.resolve_offset(bindings),
            default: self.default.as_ref().map(|d| d.resolve_offset(bindings)),
            ..self.clone()
        }
    }
}

impl WithPos for EventBind {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

impl EventBind {
    pub fn new(event: Id, delay: TimeSub, default: Option<ConcTime>) -> Self {
        Self {
            event,
            delay,
            default,
            pos: GPosIdx::UNKNOWN,
        }
    }
}

impl Display for EventBind {
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
pub struct Signature {
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
    pub events: Vec<EventBind>,
    /// Constraints on the abstract variables in the signature
    pub constraints: Vec<Constraint>,
    /// All the input/output ports.
    ports: Vec<PortDef>,
    /// Index of the first output port in the ports vector
    outputs_idx: usize,
}

impl Signature {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Id,
        params: Vec<Id>,
        events: Vec<EventBind>,
        unannotated_ports: Vec<(Id, u64)>,
        interface_signals: Vec<InterfaceDef>,
        mut inputs: Vec<PortDef>,
        mut outputs: Vec<PortDef>,
        constraints: Vec<Constraint>,
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
    pub fn inputs(&self) -> impl Iterator<Item = &PortDef> {
        self.ports[..self.outputs_idx].iter()
    }
    /// Outputs of this signature
    pub fn outputs(&self) -> impl Iterator<Item = &PortDef> {
        self.ports[self.outputs_idx..].iter()
    }
    /// Iterator over all the ports of this signature
    pub fn ports(&self) -> impl Iterator<Item = &PortDef> {
        self.ports.iter()
    }

    /// Find the delay associoated with an event
    pub fn get_event(&self, event: &Id) -> &EventBind {
        self.events
            .iter()
            .find(|eb| eb.event == event)
            .unwrap_or_else(|| {
                panic!("Event {} not found in signature:\n{}", event, self.name)
            })
    }
    /// Get a port using its name
    pub fn get_port(&self, port: &Id) -> PortDef {
        self.ports
            .iter()
            .find(|p| p.name == *port)
            .cloned()
            .or_else(|| {
                self.interface_signals.iter().find_map(|id| {
                    if id.name == *port {
                        Some(id.clone().into())
                    } else {
                        None
                    }
                })
            })
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

    /// Iterate over all phantom events. A phantom event is an event that does not have a corresponding interface signal.
    pub fn phantom_events(&self) -> impl Iterator<Item = Id> + '_ {
        self.events()
            .filter(move |event| self.get_interface(event).is_none())
    }

    /// Constraints for well formed
    fn constraints(&self) -> impl Iterator<Item = Constraint> + '_ {
        self.inputs()
            .chain(self.outputs())
            .flat_map(|mpd| mpd.liveness.well_formed())
    }

    /// Construct an event binding from this Signature's events and the given
    /// arguments.
    pub fn event_binding(&self, args: &[ConcTime]) -> Binding<ConcTime> {
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
    pub fn param_binding(&self, args: &[Width]) -> Binding<Width> {
        debug_assert!(
            self.params.len() == args.len(),
            "Insuffient params for signature, required {} got {}",
            self.params.len(),
            args.len(),
        );

        Binding::new(self.params.iter().cloned().zip(args.iter().cloned()))
    }
}

impl Signature {
    pub fn map<F>(self, f: F) -> Signature
    where
        F: Fn(Width) -> Width,
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

impl Signature {
    /// Constraints generated to ensure that a signature is well-formed.
    /// 1. Ensure that all the intervals are well formed
    /// 2. Ensure for each interval that mentions event `E` in its start time, the @interface
    ///    signal for `E` pulses less often than the length of the interval itself.
    pub fn well_formed(&self) -> impl Iterator<Item = Constraint> + '_ {
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

        evs
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
            .chain(self.constraints())
    }

    pub fn resolve_offset(&self, args: &[Width]) -> Signature {
        let binding: Binding<Width> = self.param_binding(args);

        let resolved = Signature {
            params: vec![],
            ports: self
                .ports
                .iter()
                .map(|pd| pd.resolve_offset(&binding))
                .collect_vec(),
            events: self
                .events
                .iter()
                .map(|eb| eb.resolve_offset(&binding))
                .collect_vec(),
            constraints: self
                .constraints
                .iter()
                .map(|c| c.resolve_offset(&binding))
                .collect_vec(),
            ..self.clone()
        };

        resolved
    }

    pub fn resolve_event(&self, bindings: &Binding<ConcTime>) -> Self {
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
}

impl Display for Signature {
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
impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}
