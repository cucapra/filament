use crate::core;

type TR = core::Time<u64>;

pub type Component = core::Component<TR>;
pub type Namespace = core::Namespace<TR>;
pub type PortDef<W> = core::PortDef<TR, W>;
pub type Signature<W> = core::Signature<TR, W>;
pub type Constraint = core::Constraint<TR>;
pub type ConstraintBase = core::OrderConstraint<TR>;
pub type Command = core::Command<TR>;
pub type Invoke = core::Invoke<TR>;
pub type Interval = core::Range<TR>;
pub type Range = core::Range<TR>;
pub type Port = core::Port;
pub type Guard = core::Guard;
pub type Instance = core::Instance;
pub type Connect = core::Connect;
pub type Fsm = core::Fsm;
pub type Id = core::Id;
pub type InterfaceDef = core::InterfaceDef;
pub type PortType = core::PortType;
pub type PortParam = core::PortParam;
pub type EventBind = core::EventBind<TR>;
