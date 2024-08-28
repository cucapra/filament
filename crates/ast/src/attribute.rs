use enum_map::{Enum, EnumMap};
use enumflags2::{bitflags, BitFlags};
use strum_macros::EnumString;

/// An attribute that accepts a numeric value
#[derive(Enum, Clone, Copy, PartialEq, EnumString)]
pub enum NumAttr {
    /// Tells the compiler to use a slow FSM design with a given II
    #[strum(serialize = "slow_fsm")]
    SlowFSM,
}

/// An flag attribute
#[bitflags]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, EnumString)]
pub enum BoolAttr {
    /// This is a toplevel component
    TopLevel,
}

/// Represents a single attribute. This is a private enum that is used during
/// parsing to collect all attributes before creating the [Attributes] struct.
pub enum Attr {
    Bool(BoolAttr),
    Num(NumAttr, u64),
}

/// A set of attributes attached to a component
#[derive(Default, Clone)]
pub struct Attributes {
    bool_attrs: BitFlags<BoolAttr>,
    num_attrs: EnumMap<NumAttr, Option<u64>>,
}

impl Attributes {
    pub fn new(attrs: impl Iterator<Item = Attr>) -> Self {
        let mut bool_attrs = BitFlags::empty();
        let mut num_attrs = EnumMap::default();

        for attr in attrs {
            match attr {
                Attr::Bool(b) => bool_attrs |= b,
                Attr::Num(n, v) => num_attrs[n] = Some(v),
            }
        }

        Self {
            bool_attrs,
            num_attrs,
        }
    }

    /// Check whether a given flag is set
    pub fn has_flag(&self, flag: BoolAttr) -> bool {
        self.bool_attrs.contains(flag)
    }

    /// Set a flag
    pub fn set_flag(&mut self, flag: BoolAttr) {
        self.bool_attrs.insert(flag);
    }

    /// Unset a flag
    pub fn unset_flag(&mut self, flag: BoolAttr) {
        self.bool_attrs.remove(flag);
    }

    /// Get the value of a numeric attribute
    pub fn get_numeric(&self, attr: NumAttr) -> Option<u64> {
        self.num_attrs[attr]
    }

    /// Set the value of a numeric attribute
    pub fn set_numeric(&mut self, attr: NumAttr, value: Option<u64>) {
        self.num_attrs[attr] = value;
    }
}
