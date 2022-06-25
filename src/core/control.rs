use std::fmt::Display;

use itertools::Itertools;

use crate::errors::{self, Error, FilamentResult};

use super::Id;

pub enum Port {
    ThisPort(Id),
    CompPort { comp: Id, name: Id },
    Constant(u64),
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Port::ThisPort(p) => write!(f, "{}", p),
            Port::CompPort { comp, name } => write!(f, "{}.{}", comp, name),
            Port::Constant(n) => write!(f, "{}", n),
        }
    }
}

/// Command in a component
pub enum Command<T> {
    Invoke(Invoke<T>),
    Instance(Instance),
    Connect(Connect),
    Fsm(Fsm),
}

impl<T> From<Connect> for Command<T> {
    fn from(v: Connect) -> Self {
        Self::Connect(v)
    }
}

impl<T> From<Instance> for Command<T> {
    fn from(v: Instance) -> Self {
        Self::Instance(v)
    }
}

impl<T> From<Invoke<T>> for Command<T> {
    fn from(v: Invoke<T>) -> Self {
        Self::Invoke(v)
    }
}

impl<T: Display> Display for Command<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Invoke(inv) => write!(f, "{}", inv),
            Command::Instance(ins) => write!(f, "{}", ins),
            Command::Connect(con) => write!(f, "{}", con),
            Command::Fsm(fsm) => write!(f, "{}", fsm),
        }
    }
}

/// A new component instance
pub struct Instance {
    /// Name of the instance.
    pub name: Id,
    /// Name of the component
    pub component: Id,
}
impl std::fmt::Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} := new {}", self.name, self.component)
    }
}

/// An Invocation
pub struct Invoke<T> {
    /// Name of the variable being assigned
    pub bind: Id,

    /// Name of the component being invoked
    pub comp: Id,

    /// Abstract variables used for this invocation
    pub abstract_vars: Vec<T>,

    /// Assignment for the ports
    pub ports: Option<Vec<Port>>,

    /// Source location of the invocation
    pos: Option<errors::Span>,
}
impl<T> Invoke<T> {
    pub fn new(
        bind: Id,
        comp: Id,
        abstract_vars: Vec<T>,
        ports: Option<Vec<Port>>,
    ) -> Self {
        Self {
            bind,
            comp,
            abstract_vars,
            ports,
            pos: None,
        }
    }
}
impl<T: Display> Display for Invoke<T> {
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
                "{} := {}<{}>({});",
                self.bind,
                self.comp,
                abs,
                ports
                    .iter()
                    .map(|port| port.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        } else {
            write!(f, "{} := invoke {}<{}>;", self.bind, self.comp, abs)
        }
    }
}
impl<T> errors::WithPos for Invoke<T> {
    /// Attach a position to this node
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }
    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

/// A Guard expression
pub enum Guard {
    Or(Box<Guard>, Box<Guard>),
    Port(Port),
}
impl std::fmt::Display for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Guard::Or(g1, g2) => write!(f, "{} | {}", g1, g2),
            Guard::Port(p) => write!(f, "{}", p),
        }
    }
}

/// A Connection between ports
pub struct Connect {
    /// Destination port
    pub dst: Port,

    /// Source port
    pub src: Port,

    /// Optional guard expression.
    pub guard: Option<Guard>,

    /// Source location of the invocation
    pos: Option<errors::Span>,
}

impl Connect {
    pub fn new(dst: Port, src: Port, guard: Option<Guard>) -> Self {
        Self {
            dst,
            src,
            guard,
            pos: None,
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
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

/// A when statement executes its body when the provided `port` rises.
/// It also binds the `time_var` in the body to the time when the `port` rose.
pub struct When<T> {
    pub time: T,
    pub commands: Vec<Command<T>>,
}
impl<T: Display> Display for When<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "when {} {{ ", self.time)?;
        for cmd in &self.commands {
            write!(f, "{}; ", cmd)?;
        }
        write!(f, "}}")
    }
}

/// Representation of a finite state machine
pub struct Fsm {
    /// Name of the FSM
    pub name: Id,

    /// Number of states in the FSM
    pub states: u64,

    /// Signal that triggers the FSM
    pub trigger: Port,

    pos: Option<errors::Span>,
}

impl Fsm {
    pub fn new(name: Id, states: u64, trigger: Port) -> Self {
        Self {
            name,
            states,
            trigger,
            pos: None,
        }
    }

    /// Returns the state associated with the FSM port
    pub fn state(port: &Id) -> FilamentResult<u64> {
        let split = port.id.split('_').collect_vec();
        if split.len() == 2 {
            let mb_idx = split[1].parse::<u64>();
            if let Ok(idx) = mb_idx {
                return Ok(idx);
            }
        }
        Err(Error::malformed(format!(
            "Port cannot be converted into FSM state: {}",
            port
        )))
    }

    /// Return the port associated with the given state
    pub fn port(&self, state: u64) -> Port {
        Port::CompPort {
            comp: self.name.clone(),
            name: format!("_{}", state).into(),
        }
    }
}

impl errors::WithPos for Fsm {
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

impl std::fmt::Display for Fsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fsm {}[{}]({})", self.name, self.states, self.trigger)
    }
}
