use super::{
    Access, ExprIdx, Fact, InstIdx, InvIdx, ParamIdx, PropIdx, TimeIdx,
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
    // NOTE(rachit): We'll probably need this some day.
    // comp: CompIdx,
    pub params: Box<[ExprIdx]>,
}

#[derive(Clone, PartialEq, Eq)]
/// A connection between two ports
pub struct Connect {
    pub src: Access,
    pub dst: Access,
}

#[derive(Clone, PartialEq, Eq)]
/// An invocation of a component
/// Unlike in the AST, invocations are completely desuarged and do not have any
/// ports.
/// The ports are represented as connections.
pub struct Invoke {
    pub inst: InstIdx,
    pub events: Box<[TimeIdx]>,
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
