use super::ScopeMap;
use crate::ast::{self, Id, Signature};
use crate::ir::{
    self, CompIdx, Component, Ctx, DenseIndexInfo, EventIdx, PortIdx,
};
use crate::utils;
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone)]
/// The signature of component.
///
/// A signature defines the ports which are added to the component instantiating
/// the signature.
pub struct Sig {
    pub idx: CompIdx,
    pub events: Vec<EventIdx>,
    pub inputs: Vec<PortIdx>,
    pub outputs: Vec<PortIdx>,
    pub raw_params: Vec<ast::ParamBind>,
    pub raw_events: Vec<ast::EventBind>,
    pub raw_inputs: Vec<ast::Loc<ast::PortDef>>,
    pub raw_outputs: Vec<ast::PortDef>,
    pub param_cons: Vec<ast::Loc<ast::OrderConstraint<ast::Expr>>>,
    pub event_cons: Vec<ast::Loc<ast::OrderConstraint<ast::Time>>>,
}

impl Sig {
    pub fn new(idx: CompIdx, comp: &Component, sig: &Signature) -> Self {
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
    rev_map: HashMap<CompIdx, Id>,
}

impl SigMap {
    pub fn get(&self, id: &Id) -> Option<&Sig> {
        self.map.get(id)
    }

    pub fn get_idx(&self, idx: CompIdx) -> Option<&Sig> {
        self.rev_map.get(&idx).and_then(|id| self.get(id))
    }
}

impl std::ops::Index<&Id> for SigMap {
    type Output = Sig;

    fn index(&self, id: &Id) -> &Self::Output {
        self.get(id).unwrap()
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

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
/// A custom struct used to index ports
pub(super) enum InvPort {
    Sig(ir::Direction, Id),
    Inv(ir::InvIdx, ir::Direction, Id),
    Local(Id),
}

impl InvPort {
    fn name(&self) -> &Id {
        match self {
            Self::Sig(_, id) | Self::Inv(_, _, id) | Self::Local(id) => id,
        }
    }
}

impl std::fmt::Display for InvPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sig(dir, id) => write!(f, "{}({})", dir, id),
            Self::Inv(inv, dir, id) => write!(f, "{}({} in {})", dir, id, inv),
            Self::Local(id) => write!(f, "{}", id),
        }
    }
}

/// Context used while building the IR.
pub(super) struct BuildCtx<'prog> {
    comp: ir::Component,
    pub sigs: &'prog SigMap,

    // Mapping from names of instance to (<parameter bindings>, <component name>).
    // We keep around the parameter bindings as [ast::Expr] because we need to resolve
    // port definition in invokes using them.
    pub inst_to_sig:
        DenseIndexInfo<ir::Instance, (Rc<utils::Binding<ast::Expr>>, Id)>,

    // Mapping from names to IR nodes.
    pub event_map: ScopeMap<ir::Event>,
    pub inst_map: ScopeMap<ir::Instance>,

    inv_map: ScopeMap<ir::Invoke>,
    port_map: ScopeMap<ir::Port, InvPort>,
    param_map: ScopeMap<ir::Param>,

    /// Index for generating unique names
    name_idx: u32,
}

impl<'prog> BuildCtx<'prog> {
    pub fn new(comp: ir::Component, sigs: &'prog SigMap) -> Self {
        Self {
            comp,
            sigs,
            name_idx: 0,
            param_map: ScopeMap::new(),
            event_map: ScopeMap::new(),
            port_map: ScopeMap::new(),
            inst_map: ScopeMap::new(),
            inv_map: ScopeMap::new(),
            inst_to_sig: DenseIndexInfo::default(),
        }
    }

    #[inline]
    /// Get a mutable reference to current component
    pub fn comp(&mut self) -> &mut ir::Component {
        &mut self.comp
    }

    #[inline]
    // takes the component from the builder
    pub fn take(self) -> ir::Component {
        self.comp
    }

    /// Generate a unique, new name for unused parameters
    pub fn gen_name(&mut self) -> Id {
        let name = format!("_{}", self.name_idx);
        self.name_idx += 1;
        Id::from(name)
    }

    /// Push a new scope level
    #[inline]
    fn push(&mut self) {
        self.param_map.push();
        self.event_map.push();
        self.port_map.push();
        self.inst_map.push();
        self.inv_map.push();
    }

    /// Pop the last scope level
    #[inline]
    fn pop(&mut self) {
        self.param_map.pop();
        self.event_map.pop();
        self.port_map.pop();
        self.inst_map.pop();
        self.inv_map.pop();
    }

    /// Perform some action within a new scope
    pub fn with_scope<T, F>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.push();
        let out = f(self);
        self.pop();
        out
    }

    pub fn get_param(&mut self, name: &Id) -> Option<ir::ParamIdx> {
        self.param_map.get(name).copied()
    }

    pub fn add_param(&mut self, name: Id, param: ir::ParamIdx) {
        self.param_map.insert(name, param);
    }

    /// Add a new port to the ctx
    pub fn add_port(&mut self, name: Id, port: PortIdx) {
        let owner = self.comp.get(port).owner.clone();
        let owner = match owner {
            ir::PortOwner::Sig { dir } => InvPort::Sig(dir, name),
            ir::PortOwner::Inv { inv, dir, .. } => InvPort::Inv(inv, dir, name),
            ir::PortOwner::Local => InvPort::Local(name),
        };
        self.port_map.insert(owner, port);
    }

    /// Find a port and return None if it is not found
    pub fn find_port(&mut self, port: &InvPort) -> Option<PortIdx> {
        self.port_map.get(port).copied()
    }

    /// Get a port and panic out if it is not found
    pub fn get_port(&mut self, port: &InvPort) -> PortIdx {
        // Clone here because we use owner in the error message below
        if let Some(idx) = self.find_port(port) {
            return idx;
        }

        let name = port.name();

        // We are going to error out anyways so attempt to find the scope for
        // *any* port with the same name but a different port owner.
        let other = self
            .port_map
            .as_flat_vec()
            .into_iter()
            .find_map(|(p, _)| if p.name() == name { Some(p) } else { None });
        let mut msg = format!("Port `{port}' not found");
        if let Some(port) = other {
            msg.push_str(&format!(
                ". However, a port with the same name exists: `{port}'",
            ));
        }
        msg.push_str(&format!(
            ". Component:\n{comp}",
            comp = ir::Printer::comp_str(&self.comp)
        ));
        unreachable!("{msg}")
    }

    pub fn add_inv(&mut self, name: Id, inv: ir::InvIdx) {
        self.inv_map.insert(name, inv);
    }

    pub fn get_inv(&mut self, name: Id) -> ir::InvIdx {
        *self.inv_map.get(&name).unwrap_or_else(|| {
            unreachable!(
                "Invoke `{name}' not found. Component:\n{comp}",
                name = name,
                comp = ir::Printer::comp_str(&self.comp)
            )
        })
    }
}
