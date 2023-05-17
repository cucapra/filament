use super::{
    idxs::CompIdx, CmdIdx, ConIdx, EventIdx, ExprIdx, IfIdx, InstIdx, InvIdx,
    LoopIdx, ParamIdx, PortIdx,
};

#[derive(Clone, PartialEq, Eq)]
/// A flattened and minimized representation of the control flow graph.
/// Bundle definitions and facts are removed during the process of compilation to the IR.
pub enum Command {
    Connect(ConIdx),
    Instance(InstIdx),
    Invoke(InvIdx),
    ForLoop(LoopIdx),
    If(IfIdx),
}

#[derive(Clone, PartialEq, Eq)]
/// An instantiated component
pub struct Instance {
    comp: CompIdx,
    params: Box<[ParamIdx]>,
}

#[derive(Clone, PartialEq, Eq)]
/// A connection between two ports
pub struct Connect {
    src: PortIdx,
    dst: PortIdx,
}

#[derive(Clone, PartialEq, Eq)]
/// An invocation of a component
/// Unlike in the AST, invocations are completely desuarged and do not have any
/// ports.
/// The ports are represented as connections.
pub struct Invoke {
    comp: InstIdx,
    events: Box<[EventIdx]>,
}

#[derive(Clone, PartialEq, Eq)]
/// A loop over a range of numbers
pub struct Loop {
    index: ParamIdx,
    start: ExprIdx,
    end: ExprIdx,
    body: Box<[CmdIdx]>,
}

#[derive(Clone, PartialEq, Eq)]
/// A conditional statement
pub struct If {
    cond: ExprIdx,
    then: Box<[CmdIdx]>,
    alt: Box<[CmdIdx]>,
}
