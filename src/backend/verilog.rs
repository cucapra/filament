use crate::core;

pub enum PortDir {
    Input,
    Output,
}

pub struct PortDef {
    pub name: core::Id,
    pub width: u64,
    pub direction: PortDir,
}
