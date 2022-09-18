use crate::errors::WithPos;
use crate::{
    errors::{self, Error, FilamentResult},
    event_checker::ast,
};
use std::collections::HashMap;

/// An Instance that has been resolved
pub enum ResolvedInstance<'a> {
    Bound {
        sig: &'a ast::Signature<ast::PortParam>,
        binds: Vec<u64>,
    },
    Concrete {
        sig: &'a ast::Signature<u64>,
        pos: Option<errors::Span>,
    },
}

impl<'a> ResolvedInstance<'a> {
    pub fn bound(
        sig: &'a ast::Signature<ast::PortParam>,
        binds: Vec<u64>,
    ) -> Self {
        Self::Bound { sig, binds }
    }

    pub fn concrete(sig: &'a ast::Signature<u64>) -> Self {
        Self::Concrete { sig, pos: None }
    }
}
impl<'a> From<&'a ast::Signature<u64>> for ResolvedInstance<'a> {
    fn from(sig: &'a ast::Signature<u64>) -> Self {
        Self::Concrete { sig, pos: None }
    }
}

impl<'a> ResolvedInstance<'a> {
    // Return the abstract variables defined by the signature of this instance.
    pub fn abstract_vars(&self) -> &[ast::Id] {
        match self {
            ResolvedInstance::Bound { sig, .. } => &sig.abstract_vars,
            ResolvedInstance::Concrete { sig, .. } => &sig.abstract_vars,
        }
    }

    pub fn input_names(&self) -> Vec<ast::Id> {
        match self {
            ResolvedInstance::Bound { sig, .. } => {
                sig.inputs.iter().map(|pd| pd.name.clone()).collect()
            }
            ResolvedInstance::Concrete { sig, .. } => {
                sig.inputs.iter().map(|pd| pd.name.clone()).collect()
            }
        }
    }

    pub fn interface_name(&self) -> Vec<ast::Id> {
        match self {
            ResolvedInstance::Bound { sig, .. } => sig
                .interface_signals
                .iter()
                .map(|id| id.name.clone())
                .collect(),
            ResolvedInstance::Concrete { sig, .. } => sig
                .interface_signals
                .iter()
                .map(|id| id.name.clone())
                .collect(),
        }
    }

    pub fn get_interface(&self, event: &ast::Id) -> Option<&ast::InterfaceDef> {
        match self {
            ResolvedInstance::Bound { sig, .. } => sig.get_interface(event),
            ResolvedInstance::Concrete { sig, .. } => sig.get_interface(event),
        }
    }

    pub fn resolve(&self) -> ast::Signature<u64> {
        match self {
            ResolvedInstance::Bound { sig, binds, .. } => sig
                .resolve(binds)
                .unwrap_or_else(|_| panic!("Failed to resolve signature")),
            ResolvedInstance::Concrete { sig, .. } => (*sig).clone(),
        }
    }

    pub fn binding(&self, abs: &'a [ast::TimeRep]) -> ast::Binding {
        match self {
            ResolvedInstance::Bound { sig, .. } => sig.binding(abs),
            ResolvedInstance::Concrete { sig, .. } => sig.binding(abs),
        }
    }
}

/// Environment to store the current set of bindings
pub struct Bindings<'a> {
    /// Signatures for external definitions
    ext_sigs: HashMap<ast::Id, &'a ast::Signature<ast::PortParam>>,
    /// Signatures for components
    comps: Vec<ast::Component>,
}
impl<'a> Bindings<'a> {
    pub fn new(
        ext_sigs: HashMap<ast::Id, &'a ast::Signature<ast::PortParam>>,
    ) -> Self {
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
        binds: &[u64],
    ) -> FilamentResult<ResolvedInstance> {
        if let Some(sig) = self.ext_sigs.get(name) {
            Ok(ResolvedInstance::bound(sig, binds.to_vec()))
        } else {
            self.comps
                .iter()
                .find(|c| c.sig.name == name)
                .map(|comp| ResolvedInstance::concrete(&comp.sig))
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
        binds: &[u64],
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
