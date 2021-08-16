use super::{Id, IntervalTime};

pub enum Port {
    ThisPort(Id),
    CompPort { comp: Id, name: Id },
    Constant(u64),
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
