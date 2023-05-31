use std::fmt;

use super::{
    Access, CompIdx, EventIdx, ExprIdx, Fact, InstIdx, InvIdx, ParamIdx,
    PortIdx, PropIdx, TimeIdx,
};

#[derive(Clone, PartialEq, Eq)]
/// A flattened and minimized representation of the control flow graph.
/// Bundle definitions and facts are removed during the process of compilation to the IR.
pub enum Command {
    Instance(InstIdx),
    Invoke(InvIdx),
    Connect(Connect),
    ForLoop(Loop),
    If(If),
    Fact(Fact),
    EventBind(EventBind),
}
impl From<InstIdx> for Command {
    fn from(idx: InstIdx) -> Self {
        Command::Instance(idx)
    }
}
impl From<InvIdx> for Command {
    fn from(idx: InvIdx) -> Self {
        Command::Invoke(idx)
    }
}
impl From<Connect> for Command {
    fn from(con: Connect) -> Self {
        Command::Connect(con)
    }
}
impl From<EventBind> for Command {
    fn from(bind: EventBind) -> Self {
        Command::EventBind(bind)
    }
}
impl From<Loop> for Command {
    fn from(loop_: Loop) -> Self {
        Command::ForLoop(loop_)
    }
}
impl From<If> for Command {
    fn from(if_: If) -> Self {
        Command::If(if_)
    }
}
impl From<Fact> for Command {
    fn from(fact: Fact) -> Self {
        Command::Fact(fact)
    }
}

#[derive(Clone, PartialEq, Eq)]
/// An instantiated component
pub struct Instance {
    /// The component being instantiated
    pub comp: CompIdx,
    /// The parameters used in the binding of this instance
    pub params: Box<[ExprIdx]>,
}

impl fmt::Display for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[", self.comp)?;
        for (i, param) in self.params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", param)?;
        }
        write!(f, "]")
    }
}

#[derive(Clone, PartialEq, Eq)]
/// A connection between two ports
pub struct Connect {
    pub src: Access,
    pub dst: Access,
}
impl fmt::Display for Connect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} = {}", self.dst, self.src)
    }
}

#[derive(Clone, PartialEq, Eq)]
/// An invocation of a component
/// Unlike in the AST, invocations are completely desuarged and do not have any
/// ports.
/// The ports are represented as connections.
pub struct Invoke {
    /// The instance being invoked
    pub inst: InstIdx,
    // The ports defined by this invocation
    pub ports: Vec<PortIdx>,
}

#[derive(Clone, PartialEq, Eq)]
/// A loop over a range of numbers
pub struct Loop {
    pub index: ParamIdx,
    pub start: ExprIdx,
    pub end: ExprIdx,
    pub body: Vec<Command>,
}

#[derive(Clone, PartialEq, Eq)]
/// A conditional statement
pub struct If {
    pub cond: PropIdx,
    pub then: Vec<Command>,
    pub alt: Vec<Command>,
}

#[derive(Clone, PartialEq, Eq)]
/// Binding for an event argument of an invocation
pub struct EventBind {
    pub event: EventIdx,
    pub arg: TimeIdx,
}
