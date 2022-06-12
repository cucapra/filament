use crate::core;

type TR = super::IntervalTime;
type L = ();

pub type Component = core::Component<TR, L>;
pub type Namespace = core::Namespace<TR, L>;
pub type PortDef = core::PortDef<TR>;
pub type Signature = core::Signature<TR>;
pub type Constraint = core::Constraint<TR>;
pub type Command = core::Command<TR, L>;
pub type Invoke = core::Invoke<TR, L>;
pub type Interval = core::Interval<TR>;
pub type Range = core::Range<TR>;
pub type Port = core::Port<L>;
pub type Guard = core::Guard<L>;
pub type Instance = core::Instance;
pub type Connect = core::Connect<L>;
pub type Fsm = core::Fsm<L>;
pub type OrderOp = core::OrderOp;
pub type Id = core::Id;
pub type PortType = core::PortType;
