use super::{
    Expr, Id, InterfaceDef, Loc, OrderConstraint, PortDef, Time, TimeSub,
};
use crate::utils::{Binding, GPosIdx};

#[derive(Clone)]
/// An event variable bound in the signature
pub struct EventBind {
    pub event: Loc<Id>,
    pub delay: Loc<TimeSub>,
    pub default: Option<Time>,
}

impl EventBind {
    pub fn resolve_event(self, bindings: &Binding<Time>) -> Self {
        Self {
            delay: self.delay.map(|e| e.resolve_event(bindings)),
            default: self.default.map(|d| d.resolve_event(bindings)),
            ..self
        }
    }

    pub fn resolve_exprs(self, bindings: &Binding<Expr>) -> Self {
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

#[derive(Clone)]
/// A parameter bound in the signature
pub struct ParamBind {
    pub param: Loc<Id>,
    pub default: Option<Expr>,
}

impl ParamBind {
    pub fn new(param: Loc<Id>, default: Option<Expr>) -> Self {
        Self { param, default }
    }

    pub fn name(&self) -> Id {
        self.param.copy()
    }

    pub fn pos(&self) -> GPosIdx {
        self.param.pos()
    }
}

impl From<Loc<Id>> for ParamBind {
    fn from(value: Loc<Id>) -> Self {
        ParamBind::new(value, None)
    }
}

impl From<Id> for ParamBind {
    fn from(value: Id) -> Self {
        ParamBind::from(Loc::unknown(value))
    }
}

/// The signature of a component definition
#[derive(Default, Clone)]
pub struct Signature {
    /// Name of the component
    pub name: Loc<Id>,
    /// Parameters for the Signature
    pub params: Vec<Loc<ParamBind>>,
    /// Parameters bound in the signature binding. These always have a default value.
    pub sig_bindings: Vec<Loc<ParamBind>>,
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
    pub ports: Vec<Loc<PortDef>>,
    /// Index of the first output port in the ports vector
    outputs_idx: usize,
}

impl Signature {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: Loc<Id>,
        params: Vec<Loc<ParamBind>>,
        events: Vec<Loc<EventBind>>,
        unannotated_ports: Vec<(Id, u64)>,
        interface_signals: Vec<InterfaceDef>,
        mut inputs: Vec<Loc<PortDef>>,
        mut outputs: Vec<Loc<PortDef>>,
        param_constraints: Vec<Loc<OrderConstraint<Expr>>>,
        event_constraints: Vec<Loc<OrderConstraint<Time>>>,
        sig_bindings: Vec<Loc<ParamBind>>,
    ) -> Self {
        let outputs_idx = inputs.len();
        inputs.append(&mut outputs);
        Self {
            name,
            params,
            sig_bindings,
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
    /// Params bound by the signature
    pub fn params(&self) -> impl Iterator<Item = Loc<Id>> + '_ {
        self.params.iter().map(|eb| &eb.param).cloned()
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

    pub fn sig_bindings(&self) -> impl Iterator<Item = Loc<Id>> + '_ {
        self.sig_bindings.iter().map(|p| &p.param).cloned()
    }
}
