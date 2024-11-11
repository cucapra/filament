use super::{Bundle, Id, Loc};

pub type PortDef = Bundle;

#[derive(Clone)]
pub struct InterfaceDef {
    /// Name of the port
    pub name: Loc<Id>,
    /// Event that this port is an evidence of
    pub event: Id,
}
impl InterfaceDef {
    pub fn new(name: Loc<Id>, event: Id) -> Self {
        Self { name, event }
    }
}
