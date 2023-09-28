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
    /// Mapping from ports in the old component to ports in the new component.
    port_map: SparseInfoMap<ir::Port, Base<ir::Port>, Underlying<ir::Port>>,
    /// Mapping from old events to new events, for resolving Foreigns
    event_map: SparseInfoMap<ir::Event, Base<ir::Event>, Underlying<ir::Event>>,
    /// Values of existentially quantified parameters
    exist_param_vals: SparseInfoMap<ir::Param, u64, Underlying<ir::Param>>,
}

impl InstanceInfo {
    pub fn add_port(&mut self, old: Underlying<ir::Port>, new: Base<ir::Port>) {
        self.port_map.push(old, new);
    }
    pub fn get_port(
        &self,
        old: Underlying<ir::Port>,
    ) -> Option<Base<ir::Port>> {
        // XXX(rachit): This assert currently gets triggered but should not.
        // assert!(!self.port_map.contains(old), "port already has a mapping");
        self.port_map.find(old).copied()
    }

    pub fn add_event(
        &mut self,
        old: Underlying<ir::Event>,
        new: Base<ir::Event>,
    ) {
        // XXX(rachit): This assert currently gets triggered but should not.
        // assert!(!self.event_map.contains(old), "event already has a mapping");
        self.event_map.push(old, new);
    }
    pub fn get_event(
        &self,
        old: Underlying<ir::Event>,
    ) -> Option<Base<ir::Event>> {
        self.event_map.find(old).copied()
    }

    /// Add binding for an existentially quantified parameter
    pub fn add_exist_val(&mut self, param: Underlying<ir::Param>, val: u64) {
        assert!(
            !self.exist_param_vals.contains(param),
            "existential parameter already has a value"
        );
        self.exist_param_vals.push(param, val);
    }

    /// Get the value for an existentially quantified parameter
    pub fn get_exist_val(&self, old: Underlying<ir::Param>) -> Option<u64> {
        self.exist_param_vals.find(old).copied()
    }

    /// Iterate over all existentially quantified parameters
    pub fn iter_exist_vals(
        &self,
    ) -> impl Iterator<Item = (Underlying<ir::Param>, u64)> + '_ {
        self.exist_param_vals.iter().map(|(ul, v)| (ul, *v))
    }
}
