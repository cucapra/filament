use crate::errors;

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
    When(When<T>),
    Instance(Instance),
    Connect(Connect),
}

impl<T> Command<T> {
    pub fn when(time: T, body: Vec<Command<T>>) -> Self {
        Command::When(When {
            time,
            commands: body,
        })
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for Command<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Invoke(inv) => write!(f, "{}", inv),
            Command::When(wh) => write!(f, "{}", wh),
            Command::Instance(ins) => write!(f, "{}", ins),
            Command::Connect(con) => write!(f, "{}", con),
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

    /// Invocation assigning to this variable
    pub rhs: Invocation<T>,
}
impl<T: std::fmt::Debug> std::fmt::Display for Invoke<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} := {}", self.bind, self.rhs)
    }
}
impl<T> errors::WithPos for Invoke<T> {
    fn copy_span(&self) -> Option<errors::Span> {
        self.rhs.copy_span()
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

    /// Attach a position to this node
    pub fn with_span(mut self, sp: errors::Span) -> Self {
        self.pos = Some(sp);
        self
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
    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

pub struct Invocation<T> {
    /// Name of the component being invoked
    pub comp: Id,

    /// Abstract variables used for this invocation
    pub abstract_vars: Vec<T>,

    /// Assignment for the ports
    pub ports: Vec<Port>,

    /// Source location of the invocation
    pos: Option<errors::Span>,
}

impl<T> errors::WithPos for Invocation<T> {
    fn copy_span(&self) -> Option<errors::Span> {
        self.pos.clone()
    }
}

impl<T> Invocation<T> {
    pub fn new(comp: Id, abstract_vars: Vec<T>, ports: Vec<Port>) -> Self {
        Self {
            comp,
            abstract_vars,
            ports,
            pos: None,
        }
    }

    /// Attach a position to this node
    pub fn with_span(mut self, sp: errors::Span) -> Self {
        self.pos = Some(sp);
        self
    }
}

impl<T: std::fmt::Debug> std::fmt::Display for Invocation<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}<{}>({})",
            self.comp,
            self.abstract_vars
                .iter()
                .map(|it| format!("{:?}", it))
                .collect::<Vec<String>>()
                .join(","),
            self.ports
                .iter()
                .map(|port| port.to_string())
                .collect::<Vec<String>>()
                .join(",")
        )
    }
}

/// A when statement executes its body when the provided `port` rises.
/// It also binds the `time_var` in the body to the time when the `port` rose.
pub struct When<T> {
    pub time: T,
    pub commands: Vec<Command<T>>,
}
impl<T: std::fmt::Debug> std::fmt::Display for When<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "when {:?} {{ ", self.time)?;
        for cmd in &self.commands {
            write!(f, "{}; ", cmd)?;
        }
        write!(f, "}}")
    }
}
