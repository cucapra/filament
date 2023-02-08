use itertools::Itertools;

use crate::errors::WithPos;
use crate::{
    ast::param as ast,
    errors::{Error, FilamentResult},
};
use std::collections::HashMap;

/// An Instance that has been resolved
pub struct ResolvedInstance<'a> {
    sig: &'a ast::Signature,
    binds: Vec<ast::PortParam>,
}

impl<'a> ResolvedInstance<'a> {
    pub fn bound(sig: &'a ast::Signature, binds: Vec<ast::PortParam>) -> Self {
        log::trace!("sig = {}, binds = {:?}", sig, binds);
        Self { sig, binds }
    }

    pub fn this(sig: &'a ast::Signature) -> Self {
        let binds = sig
            .params
            .iter()
            .map(|p| ast::PortParam::Var(p.clone()))
            .collect_vec();
        Self::bound(sig, binds)
    }
}

impl<'a> ResolvedInstance<'a> {
    pub fn sig(&self) -> &'a ast::Signature {
        self.sig
    }

    // Return the abstract variables defined by the signature of this instance.
    pub fn events(&self) -> Vec<ast::Id> {
        self.sig.events().collect()
    }

    pub fn abstract_vars(&self) -> &[ast::EventBind] {
        &self.sig.events
    }

    pub fn interface_signals(&self) -> &[ast::InterfaceDef] {
        &self.sig.interface_signals
    }

    pub fn input_names(&self) -> Vec<ast::Id> {
        self.sig.inputs().map(|pd| pd.name.clone()).collect()
    }

    pub fn interface_name(&self) -> Vec<ast::Id> {
        self.sig
            .interface_signals
            .iter()
            .map(|id| id.name.clone())
            .collect()
    }

    pub fn get_interface(&self, event: &ast::Id) -> Option<&ast::InterfaceDef> {
        self.sig.get_interface(event)
    }

    pub fn get_event(&self, event: &ast::Id) -> Option<&ast::EventBind> {
        Some(self.sig.get_event(event))
    }

    pub fn phantom_events(&self) -> Vec<ast::Id> {
        self.sig.phantom_events().collect()
    }

    pub fn resolve(&self) -> FilamentResult<ast::Signature> {
        self.sig.resolve(&self.binds).map_err(|e| {
            e.add_note(
                "Attempting to resolve signature",
                self.sig.name.copy_span(),
            )
        })
    }

    pub fn binding(&self, abs: &'a [ast::TimeRep]) -> ast::Binding {
        self.sig.binding(abs)
    }
}

/// Environment to store the current set of bindings
pub struct Bindings<'a> {
    /// Signatures for external definitions
    ext_sigs: HashMap<ast::Id, &'a ast::Signature>,
    /// Signatures for components
    comps: Vec<ast::Component>,
}
impl<'a> Bindings<'a> {
    pub fn new(ext_sigs: HashMap<ast::Id, &'a ast::Signature>) -> Self {
        Self {
            ext_sigs,
            comps: Vec::new(),
        }
    }

    /// Add a component definition to the environment
    pub fn add_component(&mut self, comp: ast::Component) {
        self.comps.push(comp);
    }

    /// Get a binding associated with a name or return error
    pub fn find_component(
        &'a self,
        name: &ast::Id,
        binds: &[ast::PortParam],
    ) -> FilamentResult<ResolvedInstance> {
        if let Some(sig) = self.ext_sigs.get(name) {
            Ok(ResolvedInstance::bound(sig, binds.to_vec()))
        } else {
            self.comps
                .iter()
                .find(|c| c.sig.name == name)
                .map(|comp| ResolvedInstance::bound(&comp.sig, binds.to_vec()))
                .ok_or_else(|| {
                    Error::undefined(name.clone(), "component")
                        .add_note("Undefined component", name.copy_span())
                })
        }
    }

    // Returns a component or panics
    pub fn get_component(
        &'a self,
        name: &ast::Id,
        binds: &[ast::PortParam],
    ) -> ResolvedInstance {
        self.find_component(name, binds)
            .unwrap_or_else(|_| panic!("Failed to find component {}", name))
    }
}
impl From<Bindings<'_>> for Vec<ast::Component> {
    fn from(bind: Bindings<'_>) -> Self {
        bind.comps
    }
}
