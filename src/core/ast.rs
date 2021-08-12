use super::Id;

pub enum TimeOp {
    Add,
    Sub,
}

pub enum IntervalTime {
    Abstract(Id),
    Concrete(u64),
    BinOp {
        op: TimeOp,
        left: Box<IntervalTime>,
        right: Box<IntervalTime>,
    },
}

pub enum IntervalType {
    Exact,
    Within,
}

pub struct Interval {
    pub tag: IntervalType,
    pub start: IntervalTime,
    pub end: IntervalTime,
}

pub struct Port {
    /// Name of the port
    pub name: Id,

    /// Liveness condition for the Port
    pub liveness: Interval,

    /// Bitwidth of the port
    pub bitwidth: u64,
}

pub struct Body;

pub struct Component {
    /// Name of the component
    pub name: Id,

    /// Names of sub-circuits used in constructing this component
    pub cells: Vec<Id>,

    /// Names of abstract variables bound by the component
    pub abstract_vars: Vec<Id>,

    /// Input ports
    pub inputs: Vec<Port>,

    /// Output ports
    pub outputs: Vec<Port>,

    /// Model for this component
    pub body: Body,
}

pub struct Namespace {
    /// Components defined in this file
    pub components: Vec<Component>,
}
