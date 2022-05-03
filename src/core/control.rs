use super::{Cell, Id, IntervalTime};

pub enum Port {
    ThisPort(Id),
    CompPort { comp: Id, name: Id },
    Constant(u64),
}

/// Command in a component
pub enum Command {
    Invoke(Invoke),
    When(When),
    Instance(Cell),
    Connect(Connect),
}

/// An Invocation
pub struct Invoke {
    /// Name of the variable being assigned
    pub bind: Id,

    /// Invocation assigning to this variable
    pub rhs: Invocation,
}

/// A Connection between ports
pub struct Connect {
    /// Destination port
    pub dst: Port,

    /// Source port
    pub src: Port,
}

pub struct Invocation {
    /// Name of the component being invoked
    pub comp: Id,

    /// Abstract variables used for this invocation
    pub abstract_vars: Vec<IntervalTime>,

    /// Assignment for the ports
    pub ports: Vec<Port>,
}

/// A when statement executes its body when the provided `port` rises.
/// It also binds the `time_var` in the body to the time when the `port` rose.
pub struct When {
    pub time: IntervalTime,
    pub commands: Vec<Command>,
}
