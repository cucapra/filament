use super::{Binding, Expr, Id, Time};
use crate::{
    errors::{self, Error, FilamentResult, WithPos},
    utils::GPosIdx,
};
use itertools::Itertools;
use std::fmt::Display;

#[derive(Clone)]
pub struct Port {
    pub typ: PortType,
    pos: GPosIdx,
}
impl Port {
    pub fn name(&self) -> Id {
        match &self.typ {
            PortType::ThisPort(n) => n.clone(),
            PortType::InvPort { name, .. } => name.clone(),
            PortType::Constant(n) => Id::from(format!("const<{}>", n)),
        }
    }
}
impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.typ)
    }
}
impl errors::WithPos for Port {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

#[derive(Clone)]
pub enum PortType {
    ThisPort(Id),
    InvPort { invoke: Id, name: Id },
    Constant(u64),
}

impl Port {
    pub fn comp(comp: Id, name: Id) -> Self {
        Port {
            typ: PortType::InvPort { invoke: comp, name },
            pos: GPosIdx::UNKNOWN,
        }
    }

    pub fn this(p: Id) -> Self {
        Port {
            typ: PortType::ThisPort(p),
            pos: GPosIdx::UNKNOWN,
        }
    }

    pub fn constant(v: u64) -> Self {
        Port {
            typ: PortType::Constant(v),
            pos: GPosIdx::UNKNOWN,
        }
    }
}

impl std::fmt::Display for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortType::ThisPort(p) => write!(f, "{}", p),
            PortType::InvPort { invoke: comp, name } => {
                write!(f, "{}.{}", comp, name)
            }
            PortType::Constant(n) => write!(f, "{}", n),
        }
    }
}

#[derive(Clone)]
/// Command in a component
pub enum Command {
    Invoke(Invoke),
    Instance(Instance),
    Connect(Connect),
    Fsm(Fsm),
}

impl From<Fsm> for Command {
    fn from(v: Fsm) -> Self {
        Self::Fsm(v)
    }
}

impl From<Connect> for Command {
    fn from(v: Connect) -> Self {
        Self::Connect(v)
    }
}

impl From<Instance> for Command {
    fn from(v: Instance) -> Self {
        Self::Instance(v)
    }
}

impl From<Invoke> for Command {
    fn from(v: Invoke) -> Self {
        Self::Invoke(v)
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Invoke(inv) => write!(f, "{}", inv),
            Command::Instance(ins) => write!(f, "{}", ins),
            Command::Connect(con) => write!(f, "{}", con),
            Command::Fsm(fsm) => write!(f, "{}", fsm),
        }
    }
}

#[derive(Clone)]
/// A new component instance
pub struct Instance {
    /// Name of the instance.
    pub name: Id,
    /// Name of the component
    pub component: Id,
    /// Bindings provided for this instance
    pub bindings: Vec<Expr>,
    /// Source position
    pos: GPosIdx,
}
impl Instance {
    pub fn new(name: Id, component: Id, bindings: Vec<Expr>) -> Self {
        Instance {
            name,
            component,
            bindings,
            pos: GPosIdx::UNKNOWN,
        }
    }
}
impl std::fmt::Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} := new {}", self.name, self.component)?;
        if !self.bindings.is_empty() {
            write!(f, "[{}]", self.bindings.iter().join(", "))
        } else {
            Ok(())
        }
    }
}
impl WithPos for Instance {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

#[derive(Clone)]
/// An Invocation
pub struct Invoke {
    /// Name of the variable being assigned
    pub name: Id,
    /// Name of the component being invoked
    pub instance: Id,
    /// Abstract variables used for this invocation
    pub abstract_vars: Vec<Time>,
    /// Assignment for the ports
    pub ports: Option<Vec<Port>>,
    /// Source location of the invocation
    pos: GPosIdx,
}

impl Invoke {
    pub fn new(
        name: Id,
        instance: Id,
        abstract_vars: Vec<Time>,
        ports: Option<Vec<Port>>,
    ) -> Self {
        Self {
            name,
            instance,
            abstract_vars,
            ports,
            pos: GPosIdx::UNKNOWN,
        }
    }

    // XXX: This can probably be removed
    pub fn bindings<I>(&self, abstract_vars: I) -> Binding<Time>
    where
        I: Iterator<Item = Id>,
    {
        Binding::new(abstract_vars.zip(self.abstract_vars.iter().cloned()))
    }
}
impl Display for Invoke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let abs = self
            .abstract_vars
            .iter()
            .map(|it| format!("{}", it))
            .collect::<Vec<String>>()
            .join(",");
        if let Some(ports) = &self.ports {
            write!(
                f,
                "{} := {}<{}>({})",
                self.name,
                self.instance,
                abs,
                ports
                    .iter()
                    .map(|port| port.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        } else {
            write!(f, "{} := invoke {}<{}>", self.name, self.instance, abs)
        }
    }
}
impl errors::WithPos for Invoke {
    /// Attach a position to this node
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }
    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

#[derive(Clone)]
/// A Guard expression
pub enum Guard {
    Or(Box<Guard>, Box<Guard>, GPosIdx),
    Port(Port),
}

impl From<Port> for Guard {
    fn from(v: Port) -> Self {
        Self::Port(v)
    }
}
impl Guard {
    pub fn or(g1: Guard, g2: Guard) -> Self {
        Guard::Or(Box::new(g1), Box::new(g2), GPosIdx::UNKNOWN)
    }
}
impl errors::WithPos for Guard {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        match self {
            Guard::Or(_, _, ref mut span) => {
                *span = sp;
                self
            }
            Guard::Port(p) => p.set_span(sp).into(),
        }
    }

    fn copy_span(&self) -> GPosIdx {
        match self {
            Guard::Or(_, _, sp) => *sp,
            Guard::Port(p) => p.copy_span(),
        }
    }
}
impl std::fmt::Display for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Guard::Or(g1, g2, _) => write!(f, "{} | {}", g1, g2),
            Guard::Port(p) => write!(f, "{}", p),
        }
    }
}

#[derive(Clone)]
/// A Connection between ports
pub struct Connect {
    /// Destination port
    pub dst: Port,

    /// Source port
    pub src: Port,

    /// Optional guard expression.
    pub guard: Option<Guard>,

    /// Source location of the invocation
    pos: GPosIdx,
}

impl Connect {
    pub fn new(dst: Port, src: Port, guard: Option<Guard>) -> Self {
        Self {
            dst,
            src,
            guard,
            pos: GPosIdx::UNKNOWN,
        }
    }
}
impl std::fmt::Display for Connect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(g) = &self.guard {
            write!(f, "{} = {} ? {}", self.dst, g, self.src)
        } else {
            write!(f, "{} = {}", self.dst, self.src)
        }
    }
}
impl errors::WithPos for Connect {
    /// Attach a position to this node
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

#[derive(Clone)]
/// Representation of a finite state machine
pub struct Fsm {
    /// Name of the FSM
    pub name: Id,
    /// Number of states in the FSM
    pub states: u64,
    /// Signal that triggers the FSM
    pub trigger: Port,
    pos: GPosIdx,
}
impl Fsm {
    pub fn new(name: Id, states: u64, trigger: Port) -> Self {
        Self {
            name,
            states,
            trigger,
            pos: GPosIdx::UNKNOWN,
        }
    }

    /// Returns the state associated with the FSM port
    pub fn state(port: &Id) -> FilamentResult<u64> {
        let split = port.id().split('_').collect_vec();
        if split.len() == 2 {
            let mb_idx = split[1].parse::<u64>();
            if let Ok(idx) = mb_idx {
                return Ok(idx);
            }
        }
        Err(Error::malformed(format!(
            "PortType cannot be converted into FSM state: {}",
            port
        )))
    }

    /// Return the port associated with the given state
    pub fn port(&self, state: u64) -> Port {
        Port::comp(self.name.clone(), format!("_{}", state).into())
    }
}

impl errors::WithPos for Fsm {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

impl Display for Fsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fsm {}[{}]({})", self.name, self.states, self.trigger)
    }
}
impl std::fmt::Debug for Fsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
