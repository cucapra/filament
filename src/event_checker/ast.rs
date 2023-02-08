use crate::core;

pub type TimeRep = core::Time<u64>;
pub type TimeSub = core::TimeSub<TimeRep>;

pub type Component = core::Component<TimeRep>;
pub type Namespace = core::Namespace<TimeRep>;
pub type PortDef<W> = core::PortDef<TimeRep, W>;
pub type Signature<W> = core::Signature<TimeRep, W>;
pub type Constraint = core::Constraint<TimeRep>;
pub type CBT = core::OrderConstraint<TimeRep>;
pub type CBS = core::OrderConstraint<TimeSub>;
pub type Command = core::Command<TimeRep>;
pub type Invoke = core::Invoke<TimeRep>;
pub type Interval = core::Range<TimeRep>;
pub type Range = core::Range<TimeRep>;
pub type InterfaceDef = core::InterfaceDef;
pub type Port = core::Port;
pub type Guard = core::Guard;
pub type Instance = core::Instance;
pub type Connect = core::Connect;
pub type Fsm = core::Fsm;
pub type Id = core::Id;
pub type PortType = core::PortType;
pub type Binding = core::Binding<TimeRep>;
pub type PortParam = core::PortParam;
pub type EventBind = core::EventBind<TimeRep>;
