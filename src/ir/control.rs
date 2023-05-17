use super::{
    idxs::CompIdx, CmdIdx, ConIdx, EventIdx, ExprIdx, IfIdx, InstIdx, InvIdx,
    LoopIdx, ParamIdx, PortIdx,
};

/// A flattened and minimized representation of the control flow graph.
/// Bundle definitions and facts are removed during the process of compilation to the IR.
pub enum Command {
    Connect(ConIdx),
    Instance(InstIdx),
    Invoke(InvIdx),
    ForLoop(LoopIdx),
    If(IfIdx),
}

/// An instantiated component
pub struct Instance {
    comp: CompIdx,
    params: Box<[ParamIdx]>,
}

/// A connection between two ports
pub struct Connect {
    src: PortIdx,
    dst: PortIdx,
}

/// An invocation of a component
pub struct Invoke {
    comp: CompIdx,
    events: Box<[EventIdx]>,
    ports: Box<[PortIdx]>,
}

/// A loop over a range of numbers
pub struct Loop {
    index: ParamIdx,
    start: ExprIdx,
    end: ExprIdx,
    body: Box<[CmdIdx]>,
}

/// A conditional statement
pub struct If {
    cond: ExprIdx,
    then: Box<[CmdIdx]>,
    alt: Box<[CmdIdx]>,
}
