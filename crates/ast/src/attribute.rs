use strum_macros::EnumString;

/// An attribute that accepts a numeric value
#[derive(Debug, PartialEq, EnumString)]
pub enum NumAttr {
    /// Tells the compiler to use a slow FSM design with a given II
    #[strum(serialize = "slow_fsm")]
    SlowFSM,
}

/// An flag attribute
#[derive(Debug, PartialEq, EnumString)]
pub enum BoolAttr {}

/// An attribute that can be attached to a component
pub enum Attribute {
    Num(NumAttr, u64),
    Bool(BoolAttr),
}
