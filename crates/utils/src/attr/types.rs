use strum_macros::EnumString;

/// Represents a single attribute. This is a private enum that is used during
/// parsing to collect all attributes before creating the [Attributes] struct.
#[derive(Clone, Copy)]
pub enum Attr {
    Bool(BoolAttr),
    Num(NumAttr),
}

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
