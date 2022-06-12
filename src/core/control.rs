use itertools::Itertools;

use crate::errors::{self, Error, FilamentResult};

use super::Id;

/// A port in the program.
pub struct Port<L> {
    pub typ: PortType,
    pub liveness: L,
}
impl Port<()> {
    pub fn this(name: Id) -> Self {
        Port {
            typ: PortType::ThisPort(name),
            liveness: ()
        }
    }
    pub fn comp(comp: Id, name: Id) -> Self {
        Port {
            typ: PortType::CompPort { comp, name },
            liveness: ()
        }
    }
    pub fn constant(n: u64) -> Self {
        Port {
            typ: PortType::Constant(n),
            liveness: ()
        }
    }
}
impl<T> std::fmt::Display for Port<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.typ)
    }
}

pub enum PortType {
    ThisPort(Id),
    CompPort { comp: Id, name: Id },
    Constant(u64),
}
impl std::fmt::Display for PortType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PortType::ThisPort(p) => write!(f, "{}", p),
            PortType::CompPort { comp, name } => write!(f, "{}.{}", comp, name),
            PortType::Constant(n) => write!(f, "{}", n),
        }
    }
}

/// Command in a component
pub enum Command<T, L> {
    Invoke(Invoke<T, L>),
    Instance(Instance),
    Connect(Connect<L>),
    Fsm(Fsm<L>),
}

impl<T, L> From<Connect<L>> for Command<T, L> {
    fn from(v: Connect<L>) -> Self {
        Self::Connect(v)
    }
}

impl<T, L> From<Instance> for Command<T, L> {
    fn from(v: Instance) -> Self {
        Self::Instance(v)
    }
}

impl<T, L> From<Invoke<T, L>> for Command<T, L> {
    fn from(v: Invoke<T, L>) -> Self {
        Self::Invoke(v)
    }
}

impl<T: std::fmt::Debug, L> std::fmt::Display for Command<T, L> {
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
pub struct Invoke<T, L> {
    /// Name of the variable being assigned
    pub bind: Id,

    /// Name of the component being invoked
    pub comp: Id,

    /// Abstract variables used for this invocation
    pub abstract_vars: Vec<T>,

    /// Assignment for the ports
    pub ports: Option<Vec<Port<L>>>,

    /// Source location of the invocation
    pos: Option<errors::Span>,
}
impl<T, L> Invoke<T, L> {
    pub fn new(
        bind: Id,
        comp: Id,
        abstract_vars: Vec<T>,
        ports: Option<Vec<Port<L>>>,
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
impl<T: std::fmt::Debug, L> std::fmt::Display for Invoke<T, L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let abs = self
            .abstract_vars
            .iter()
            .map(|it| format!("{:?}", it))
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
impl<T, L> errors::WithPos for Invoke<T, L> {
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
pub enum Guard<L> {
    Or(Box<Guard<L>>, Box<Guard<L>>),
    Port(Port<L>),
}
impl<L> std::fmt::Display for Guard<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Guard::Or(g1, g2) => write!(f, "{} | {}", g1, g2),
            Guard::Port(p) => write!(f, "{}", p),
        }
    }
}

/// A Connection between ports
pub struct Connect<L> {
    /// Destination port
    pub dst: Port<L>,

    /// Source port
    pub src: Port<L>,

    /// Optional guard expression.
    pub guard: Option<Guard<L>>,

    /// Source location of the invocation
    pos: Option<errors::Span>,
}

impl<L> Connect<L> {
    pub fn new(dst: Port<L>, src: Port<L>, guard: Option<Guard<L>>) -> Self {
        Self {
            dst,
            src,
            guard,
            pos: None,
        }
    }
}
impl<L> std::fmt::Display for Connect<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(g) = &self.guard {
            write!(f, "{} = {} ? {}", self.dst, g, self.src)
        } else {
            write!(f, "{} = {}", self.dst, self.src)
        }
    }
}
impl<L> errors::WithPos for Connect<L> {
    /// Attach a position to this node
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

/// Representation of a finite state machine
pub struct Fsm<L> {
    /// Name of the FSM
    pub name: Id,

    /// Number of states in the FSM
    pub states: u64,

    /// Signal that triggers the FSM
    pub trigger: Port<L>,

    pos: Option<errors::Span>,
}

impl<L> Fsm<L> {
    pub fn new(name: Id, states: u64, trigger: Port<L>) -> Self {
        Self {
            name,
            states,
            trigger,
            pos: None,
        }
    }

    /// Returns the state associated with the FSM port
    pub fn state(&self, port: &Id) -> FilamentResult<u64> {
        let split = port.id.split('_').collect_vec();
        if split.len() == 2 {
            let mb_idx = split[1].parse::<u64>();
            if let Ok(idx) = mb_idx {
                return Ok(idx);
            }
        }
        Err(Error::malformed(format!(
            "Unknown port: {}.{}",
            self.name, port
        )))
    }
}

impl<L> errors::WithPos for Fsm<L> {
    fn set_span(mut self, sp: Option<errors::Span>) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

impl<L> std::fmt::Display for Fsm<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fsm {}[{}]({})", self.name, self.states, self.trigger)
    }
}
