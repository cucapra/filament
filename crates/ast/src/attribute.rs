use enum_map::{Enum, EnumMap};
use strum_macros::EnumString;

/// An attribute that accepts a numeric value
#[derive(Enum, Clone, Copy, PartialEq, EnumString)]
pub enum NumAttr {}

/// An flag attribute
#[derive(Enum, Clone, Copy, PartialEq, EnumString)]
pub enum BoolAttr {
    /// This is a toplevel component
    #[strum(serialize = "toplevel")]
    TopLevel,
    /// Do not mangle the name of this component
    #[strum(serialize = "no_mangle")]
    NoMangle,
    /// Use a counter based FSM design
    #[strum(serialize = "counter_fsm")]
    CounterFSM,
}

/// Represents a single attribute. This is a private enum that is used during
/// parsing to collect all attributes before creating the [Attributes] struct.
#[derive(Enum)]
pub enum Attr {
    Bool(BoolAttr),
    Num(NumAttr),
}

impl From<BoolAttr> for Attr {
    fn from(attr: BoolAttr) -> Self {
        Attr::Bool(attr)
    }
}

impl From<NumAttr> for Attr {
    fn from(attr: NumAttr) -> Self {
        Attr::Num(attr)
    }
}

/// A set of attributes attached to a component
#[derive(Default, Clone)]
pub struct Attributes {
    attrs: EnumMap<Attr, Option<u64>>,
}

impl Attributes {
    pub fn new(attrs: impl Iterator<Item = (Attr, u64)>) -> Self {
        Self {
            attrs: attrs.map(|(attr, v)| (attr, Some(v))).collect(),
        }
    }

    /// Get the value of an attribute.
    pub fn get(&self, attr: impl Into<Attr>) -> Option<u64> {
        self.attrs[attr.into()]
    }

    /// Set the value of an attribute
    pub fn set(&mut self, attr: impl Into<Attr>, value: u64) {
        self.attrs[attr.into()] = Some(value);
    }

    /// Remove an attribute
    pub fn remove(&mut self, attr: impl Into<Attr>) {
        self.attrs[attr.into()] = None;
    }

    /// Check if the attribute set is empty
    pub fn is_empty(&self) -> bool {
        self.attrs.iter().all(|(_, v)| v.is_none())
    }
}
