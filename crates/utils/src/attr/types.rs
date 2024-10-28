use strum_macros::EnumString;

use super::Attributes;

/// An attribute that accepts a numeric value
#[derive(Clone, Copy, PartialEq, EnumString, Eq, Hash)]
pub enum NumAttr {}

/// An flag attribute
#[derive(Clone, Copy, PartialEq, EnumString, Eq, Hash)]
pub enum BoolAttr {
    /// This is a toplevel component
    #[strum(serialize = "toplevel")]
    TopLevel,
    /// Use a counter based FSM design
    #[strum(serialize = "counter_fsm")]
    CounterFSM,
}

pub type CompAttrs = Attributes<BoolAttr, NumAttr>;
