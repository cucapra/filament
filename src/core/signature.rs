use super::{
    Constraint, Expr, Id, InterfaceDef, Loc, OrderConstraint, PortDef, Time,
    TimeSub,
};
use crate::diagnostics::Diagnostics;
use crate::utils::{self, Binding};
use itertools::Itertools;
use std::{collections::HashMap, fmt::Display};

#[derive(Clone)]
/// An event variable bound in the signature
pub struct EventBind {
    pub event: Loc<Id>,
    pub delay: Loc<TimeSub>,
    pub default: Option<Time>,
}

impl EventBind {
    fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        Self {
            delay: self.delay.map(|e| e.resolve_event(bindings)),
            default: self.default.map(|d| d.resolve_event(bindings)),
            ..self
        }
    }

    fn resolve_exprs(self, bindings: &Binding<Expr>) -> Self {
        Self {
            delay: self.delay.map(|e| e.resolve_expr(bindings)),
            default: self.default.map(|d| d.resolve_expr(bindings)),
            ..self
        }
    }
}

impl EventBind {
    pub fn new(
        event: Loc<Id>,
        delay: Loc<TimeSub>,
        default: Option<Time>,
    ) -> Self {
        Self {
            event,
            delay,
            default,
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
    pub name: Loc<Id>,
    /// Parameters for the Signature
    pub params: Vec<Id>,
    /// Unannotated ports that are threaded through by the backend
    pub unannotated_ports: Vec<(Id, u64)>,
    /// Mapping from name of signals to the abstract variable they provide
    /// evidence for.
    pub interface_signals: Vec<InterfaceDef>,
    /// Names of abstract variables bound by the component
    pub events: Vec<Loc<EventBind>>,
    /// Constraints over the parameters in the signature
    pub param_constraints: Vec<Loc<OrderConstraint<Expr>>>,
    /// Constraints over events in the signature
    pub event_constraints: Vec<Loc<OrderConstraint<Time>>>,
    /// All the input/output ports.
    ports: Vec<Loc<PortDef>>,
    /// Index of the first output port in the ports vector
    outputs_idx: usize,
}

impl Signature {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Loc<Id>,
        params: Vec<Id>,
        events: Vec<Loc<EventBind>>,
        unannotated_ports: Vec<(Id, u64)>,
        interface_signals: Vec<InterfaceDef>,
        mut inputs: Vec<Loc<PortDef>>,
        mut outputs: Vec<Loc<PortDef>>,
        param_constraints: Vec<Loc<OrderConstraint<Expr>>>,
        event_constraints: Vec<Loc<OrderConstraint<Time>>>,
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
            param_constraints,
            event_constraints,
        }
    }

    /// Events bound by the signature
    pub fn events(&self) -> impl Iterator<Item = Loc<Id>> + '_ {
        self.events.iter().map(|eb| &eb.event).cloned()
    }
    /// Inputs of this signature
    pub fn inputs(&self) -> impl Iterator<Item = &Loc<PortDef>> {
        self.ports[..self.outputs_idx].iter()
    }
    /// Outputs of this signature
    pub fn outputs(&self) -> impl Iterator<Item = &Loc<PortDef>> {
        self.ports[self.outputs_idx..].iter()
    }
    /// Iterator over all the ports of this signature
    pub fn ports(&self) -> &Vec<Loc<PortDef>> {
        &self.ports
    }

    /// Find the delay associoated with an event
    pub fn get_event(&self, event: &Id) -> &Loc<EventBind> {
        self.events
            .iter()
            .find(|eb| eb.event.inner() == event)
            .unwrap_or_else(|| {
                panic!(
                    "Event `{}' not found in component `{}'",
                    event, self.name
                )
            })
    }

    /// Find a port on the component. Returns `None` if the port does not exist.
    pub fn find_port(&self, port: &Id) -> Option<Loc<PortDef>> {
        self.ports
            .iter()
            .find(|p| p.name().inner() == port)
            .cloned()
            .or_else(|| {
                self.interface_signals.iter().find_map(|id| {
                    if *id.name.inner() == *port {
                        Some(Loc::unknown(PortDef::from(id.clone())))
                    } else {
                        None
                    }
                })
            })
    }

    /// Get a port in the signature. Panics if the port is not found.
    pub fn get_port(&self, port: &Id) -> Loc<PortDef> {
        self.find_port(port).unwrap_or_else(|| {
            panic!("Port `{}' not found in component `{}'", port, self.name)
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
    pub fn phantom_events(&self) -> impl Iterator<Item = Loc<Id>> + '_ {
        self.events()
            .filter(move |event| self.get_interface(event).is_none())
    }

    /// Constraints for well formedness
    fn portdefs_well_formed(&self) -> Vec<Loc<Constraint>> {
        self.inputs()
            .chain(self.outputs())
            .map(|mpd| {
                Loc::new(mpd.liveness().well_formed(), mpd.liveness().pos())
            })
            .collect_vec()
    }

    /// Construct an event binding from this Signature's events and the given
    /// arguments.
    pub fn event_binding(
        &self,
        args: impl IntoIterator<Item = Time>,
    ) -> Binding<Time> {
        let args = args.into_iter().collect_vec();
        debug_assert!(
            self.events
                .iter()
                .take_while(|ev| ev.default.is_none())
                .count()
                <= args.len(),
            "Insuffient events for component `{}', required at least {} got {}",
            self.name,
            self.events
                .iter()
                .take_while(|ev| ev.default.is_none())
                .count(),
            args.len(),
        );

        let mut partial_map = Binding::new(
            self.events
                .iter()
                .map(|eb| eb.event.inner())
                .cloned()
                .zip(args.iter().cloned()),
        );
        // Skip the events that have been bound
        let remaining = self
            .events
            .iter()
            .skip(args.len())
            .map(|eb| {
                let bind = eb
                    .default
                    .as_ref()
                    .unwrap()
                    .clone()
                    .resolve_event(&partial_map);
                (*eb.event.inner(), bind)
            })
            .collect();

        partial_map.extend(remaining);
        partial_map
    }

    /// Construct a parameter binding from this Signature's parameters and the
    pub fn param_binding(&self, args: Vec<Expr>) -> Binding<Expr> {
        debug_assert!(
            self.params.len() == args.len(),
            "Insuffient params for component `{}', required {} got {}",
            self.name,
            self.params.len(),
            args.len(),
        );

        Binding::new(self.params.iter().cloned().zip(args.into_iter()))
    }
}

impl Signature {
    /// Constraints generated to ensure that a signature is well-formed.
    /// 1. Ensure that all the intervals are well formed
    /// 2. Ensure for each interval that mentions event `E` in its start time, the @interface
    ///    signal for `E` pulses less often than the length of the interval itself.
    pub fn well_formed(
        &self,
        diag: &mut Diagnostics,
    ) -> Vec<utils::Obligation> {
        let mut evs: HashMap<Id, Vec<_>> = HashMap::new();

        // Compute mapping from events to intervals to mention the event in their start time.
        // In the same way use of `E` in an invoke describes how often the invoke might trigger,
        // the start time of the signal describes when the signal is triggered.
        // We do not consider the end time because that only effects the length of the signal.
        for port in self.inputs().chain(self.outputs()) {
            let delay = port.liveness().len();
            let ev = &port.liveness().start.event();
            evs.entry(*ev)
                .or_default()
                .push((delay.clone(), port.liveness().pos()))
        }

        let mut cons = self
            .portdefs_well_formed()
            .into_iter()
            .map(|c| {
                let pos = c.pos();
                c.take()
                 .obligation("interval's end must occur at least one cycle after its start")
                 .add_note(diag.add_info("cannot prove interval is well-formed", pos))
            })
            .collect_vec();

        for (ev, lens) in evs {
            let eb = self.get_event(&ev);
            let len = eb.delay.clone();
            for (port_len, port_pos) in lens {
                let con = Constraint::sub(OrderConstraint::gte(
                    len.inner().clone(),
                    port_len.clone(),
                ))
                .obligation(
                    "delay must be longer than the length of the signal",
                )
                .add_note(diag.add_info(
                    format!("signal lasts for {} cycle(s)", port_len),
                    port_pos,
                ))
                .add_note(diag.add_info(
                    format!(
                        "interface allows event to trigger every {} cycle(s)",
                        len
                    ),
                    eb.event.pos(),
                ));
                cons.push(con);
            }
        }

        cons
    }

    pub fn resolve_exprs(self, args: Vec<Expr>) -> Signature {
        let binding: Binding<Expr> = self.param_binding(args);

        Signature {
            params: vec![],
            ports: self
                .ports
                .into_iter()
                .map(|pd| pd.map(|p| p.resolve_exprs(&binding)))
                .collect_vec(),
            events: self
                .events
                .into_iter()
                .map(|eb| eb.map(|e| e.resolve_exprs(&binding)))
                .collect_vec(),
            param_constraints: self
                .param_constraints
                .into_iter()
                .map(|c| c.map(|c| c.resolve_expr(&binding)))
                .collect_vec(),
            event_constraints: self
                .event_constraints
                .into_iter()
                .map(|c| c.map(|c| c.resolve_expr(&binding)))
                .collect_vec(),
            ..self
        }
    }

    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        Self {
            ports: self
                .ports
                .into_iter()
                .map(|pd| pd.map(|p| p.resolve_event(bindings)))
                .collect(),
            event_constraints: self
                .event_constraints
                .into_iter()
                .map(|c| c.map(|c| c.resolve_event(bindings)))
                .collect(),
            events: self
                .events
                .into_iter()
                .map(|eb| eb.map(|e| e.resolve_event(bindings)))
                .collect(),
            ..self
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
                    self.params.iter().map(|p| format!("#{p}")).join(", ")
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
        if !self.event_constraints.is_empty()
            || !self.param_constraints.is_empty()
        {
            write!(
                f,
                " where {}",
                self.event_constraints
                    .iter()
                    .map(|cons| format!("{cons}"))
                    .chain(
                        self.param_constraints
                            .iter()
                            .map(|cons| format!("{cons}"))
                    )
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
