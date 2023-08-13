use crate::{
    ast::{self, Id},
    ir,
};
use std::collections::HashMap;

#[derive(Clone)]
/// The signature of component.
///
/// A signature defines the ports which are added to the component instantiating
/// the signature.
pub struct Sig {
    pub idx: ir::CompIdx,
    pub events: Vec<ir::EventIdx>,
    pub inputs: Vec<ir::PortIdx>,
    pub outputs: Vec<ir::PortIdx>,
    pub raw_params: Vec<ast::ParamBind>,
    pub raw_events: Vec<ast::EventBind>,
    pub raw_inputs: Vec<ast::Loc<ast::PortDef>>,
    pub raw_outputs: Vec<ast::PortDef>,
    pub param_cons: Vec<ast::Loc<ast::OrderConstraint<ast::Expr>>>,
    pub event_cons: Vec<ast::Loc<ast::OrderConstraint<ast::Time>>>,
}

impl Sig {
    pub fn new(
        idx: ir::CompIdx,
        comp: &ir::Component,
        sig: &ast::Signature,
    ) -> Self {
        Self {
            idx,
            events: comp.events().idx_iter().collect(),
            inputs: comp.inputs().map(|(idx, _)| idx).collect(),
            outputs: comp.outputs().map(|(idx, _)| idx).collect(),
            raw_params: sig.params.iter().map(|p| p.clone().take()).collect(),
            raw_inputs: sig.inputs().cloned().collect(),
            raw_outputs: sig.outputs().map(|p| p.clone().take()).collect(),
            raw_events: sig.events.iter().map(|e| e.clone().take()).collect(),
            param_cons: sig.param_constraints.clone(),
            event_cons: sig.event_constraints.clone(),
        }
    }
}

#[derive(Default)]
/// Track the defined signatures in the current scope.
/// Mapping from names of component to [Sig].
pub struct SigMap {
    map: HashMap<Id, Sig>,
    // XXX(rachit): This can probably be a DenseInfoMap instead
    rev_map: HashMap<ir::CompIdx, Id>,
}

impl SigMap {
    /// Gets the signature if bound
    pub fn get(&self, id: &Id) -> Option<&Sig> {
        self.map.get(id)
    }

    /// Get the signature from a component index
    pub fn get_idx(&self, idx: ir::CompIdx) -> Option<&Sig> {
        self.rev_map.get(&idx).and_then(|id| self.get(id))
    }
}

impl FromIterator<(Id, Sig)> for SigMap {
    fn from_iter<T: IntoIterator<Item = (Id, Sig)>>(iter: T) -> Self {
        let mut default = Self::default();

        for (id, sig) in iter.into_iter() {
            default.rev_map.insert(sig.idx, id);
            default.map.insert(id, sig);
        }

        default
    }
}

impl std::iter::Extend<(Id, Sig)> for SigMap {
    fn extend<I: IntoIterator<Item = (Id, Sig)>>(&mut self, iter: I) {
        let sm: SigMap = iter.into_iter().collect();
        self.rev_map.extend(sm.rev_map);
        self.map.extend(sm.map);
    }
}
