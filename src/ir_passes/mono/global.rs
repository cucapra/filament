use super::{Base, Underlying};
use fil_ir as ir;
use ir::SparseInfoMap;

#[derive(PartialEq, Eq, Hash, Clone)]
/// A key defined by a component and all of its parameters.
pub struct CompKey {
    pub comp: Underlying<ir::Component>,
    pub params: Vec<u64>,
}
impl CompKey {
    pub fn new(comp: Underlying<ir::Component>, params: Vec<u64>) -> Self {
        Self { comp, params }
    }
}
impl From<(Underlying<ir::Component>, Vec<u64>)> for CompKey {
    fn from((comp, params): (Underlying<ir::Component>, Vec<u64>)) -> Self {
        Self::new(comp, params)
    }
}

#[derive(Default)]
/// Tracks information associated with instances that have been monomorphized already.
pub struct InstanceInfo {
    pub port_map: SparseInfoMap<ir::Port, Base<ir::Port>, Underlying<ir::Port>>,
    /// Mapping from old events to new events, for resolving Foreigns
    pub event_map:
        SparseInfoMap<ir::Event, Base<ir::Event>, Underlying<ir::Event>>,
}

impl InstanceInfo {
    pub fn add_port(&mut self, old: Underlying<ir::Port>, new: Base<ir::Port>) {
        self.port_map.push(old, new);
    }
    pub fn add_event(
        &mut self,
        old: Underlying<ir::Event>,
        new: Base<ir::Event>,
    ) {
        self.event_map.push(old, new);
    }

    pub fn get_port(
        &self,
        old: Underlying<ir::Port>,
    ) -> Option<Base<ir::Port>> {
        self.port_map.find(old).copied()
    }
    pub fn get_event(
        &self,
        old: Underlying<ir::Event>,
    ) -> Option<Base<ir::Event>> {
        self.event_map.find(old).copied()
    }
}
