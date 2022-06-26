use super::IntervalTime;
use crate::core;

type TR = IntervalTime;

pub type Component = core::Component<TR>;
pub type Namespace = core::Namespace<TR>;
pub type PortDef = core::PortDef<TR>;
pub type Signature = core::Signature<TR>;
pub type Constraint = core::Constraint<TR>;
pub type Command = core::Command<TR>;
pub type Invoke = core::Invoke<TR>;
pub type Interval = core::Interval<TR>;
pub type Range = core::Range<TR>;
pub type Port = core::Port;
pub type Guard = core::Guard;
pub type Instance = core::Instance;
pub type Connect = core::Connect;
pub type Fsm = core::Fsm;
pub type OrderOp = core::OrderOp;
pub type Id = core::Id;
pub type InterfaceDef = core::InterfaceDef<TR>;
