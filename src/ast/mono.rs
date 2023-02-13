use crate::core;

pub type TimeRep = core::Time;
pub type TimeSub = core::TimeSub;
pub type Width = u64;

pub type Component = core::Component;
pub type Namespace = core::Namespace;
pub type PortDef = core::PortDef;
pub type Signature = core::Signature;
pub type Constraint = core::Constraint;
pub type CBT = core::OrderConstraint<TimeRep>;
pub type CBS = core::OrderConstraint<TimeSub>;
pub type Command = core::Command;
pub type Invoke = core::Invoke;
pub type Range = core::Range;
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
pub type EventBind = core::EventBind;
