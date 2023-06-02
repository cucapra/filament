use itertools::Itertools;

use super::ScopeMap;
use crate::ast::{self, Id};
use crate::ir::{self, CompIdx, DenseIndexInfo, PortIdx};
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
    pub params: Vec<ast::Id>,
    pub events: Vec<ast::EventBind>,
    pub inputs: Vec<ast::Loc<ast::PortDef>>,
    pub outputs: Vec<ast::PortDef>,
    pub param_cons: Vec<ast::Loc<ast::OrderConstraint<ast::Expr>>>,
    pub event_cons: Vec<ast::Loc<ast::OrderConstraint<ast::Time>>>,
}

impl From<(&ast::Signature, usize)> for Sig {
    fn from((sig, idx): (&ast::Signature, usize)) -> Self {
        Sig {
            idx: CompIdx::new(idx),
            params: sig.params.iter().map(|p| p.copy()).collect(),
            inputs: sig.inputs().cloned().collect_vec(),
            outputs: sig.outputs().map(|p| p.clone().take()).collect(),
            events: sig.events.iter().map(|e| e.clone().take()).collect(),
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
}

impl SigMap {
    pub fn insert(&mut self, id: Id, sig: Sig) {
        self.map.insert(id, sig);
    }

    pub fn get(&self, id: &Id) -> Option<&Sig> {
        self.map.get(id)
    }
}

impl std::ops::Index<&Id> for SigMap {
    type Output = Sig;

    fn index(&self, id: &Id) -> &Self::Output {
        self.get(id).unwrap()
    }
}

/// The canonical name of a port defined by (inv, port).
type InvPort = (ir::PortOwner, Id);

/// Context used while building the IR.
pub struct BuildCtx<'ctx, 'prog> {
    pub comp: &'ctx mut ir::Component,
    pub sigs: &'prog SigMap,

    // Mapping from names of instance to (<parameter bindings>, <component name>).
    // We keep around the parameter bindings as [ast::Expr] because we need to resolve
    // port definition in invokes using them.
    pub inst_to_sig:
        DenseIndexInfo<ir::Instance, (Rc<utils::Binding<ast::Expr>>, Id)>,

    // Mapping from names to IR nodes.
    pub param_map: ScopeMap<ir::Param>,
    pub event_map: ScopeMap<ir::Event>,
    pub inst_map: ScopeMap<ir::Instance>,

    inv_map: ScopeMap<ir::Invoke>,
    port_map: ScopeMap<ir::Port, InvPort>,
}
impl<'ctx, 'prog> BuildCtx<'ctx, 'prog> {
    pub fn new(comp: &'ctx mut ir::Component, sigs: &'prog SigMap) -> Self {
        Self {
            comp,
            sigs,
            param_map: ScopeMap::new(),
            event_map: ScopeMap::new(),
            port_map: ScopeMap::new(),
            inst_map: ScopeMap::new(),
            inv_map: ScopeMap::new(),
            inst_to_sig: DenseIndexInfo::default(),
        }
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

    /// Add a new port to the ctx
    pub fn add_port(&mut self, name: Id, owner: ir::PortOwner, port: PortIdx) {
        self.port_map.insert((owner, name), port);
    }

    /// Find a port and return None if it is not found
    pub fn find_port(
        &mut self,
        name: Id,
        owner: ir::PortOwner,
    ) -> Option<PortIdx> {
        self.port_map.get(&(owner, name)).copied()
    }

    /// Get a port and panic out if it is not found
    pub fn get_port(&mut self, name: Id, owner: ir::PortOwner) -> PortIdx {
        // Clone here because we use owner in the error message below
        if let Some(idx) = self.find_port(name, owner.clone()) {
            return idx;
        }

        // We are going to error out anyways so attempt to find the scope for
        // *any* port with the same name but a different port owner.
        let other = self
            .port_map
            .as_flat_vec()
            .into_iter()
            .find_map(|((o, p), _)| if p == name { Some(o) } else { None });
        let mut msg = format!("Port `{name}' not found with owner `{owner}'");
        if let Some(owner) = other {
            msg.push_str(&format!(
                ". However, a port with the same name exists with owner `{owner}'",
            ));
        }
        msg.push_str(&format!(
            ". Component:\n{comp}",
            comp = ir::Printer::comp_str(self.comp)
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
                comp = ir::Printer::comp_str(self.comp)
            )
        })
    }
}
