use super::{Id, IntervalTime};

pub enum Port {
    ThisPort(Id),
    CompPort { comp: Id, name: Id },
    Constant(u64),
}

pub enum Control {
    Assign(Assignment),
    When(When),
}

pub struct Assignment {
    /// Name of the variable being assigned
    pub bind: Id,

    /// Invocation assigning to this variable
    pub rhs: Invocation,
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
    pub port: Port,
    pub time_var: Id,
    pub body: Vec<Control>,
}
