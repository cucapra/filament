use crate::core;

pub type TimeRep = crate::core::FsmIdxs;

pub type Component = core::Component<TimeRep>;
pub type Namespace = core::Namespace<TimeRep>;
pub type PortDef = core::PortDef<TimeRep>;
pub type Signature = core::Signature<TimeRep>;
pub type Constraint = core::Constraint<TimeRep>;
pub type Command = core::Command<TimeRep>;
pub type Invoke = core::Invoke<TimeRep>;
pub type Interval = core::Interval<TimeRep>;
pub type Range = core::Range<TimeRep>;
pub type InterfaceDef = core::InterfaceDef<TimeRep>;
pub type Port = core::Port;
pub type Guard = core::Guard;
pub type Instance = core::Instance;
pub type Connect = core::Connect;
pub type Fsm = core::Fsm;
pub type OrderOp = core::OrderOp;
pub type Id = core::Id;
