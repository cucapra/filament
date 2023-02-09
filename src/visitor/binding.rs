use itertools::Itertools;

use crate::core::{self, TimeRep, WidthRep};
use crate::errors::WithPos;
use crate::errors::{Error, FilamentResult};
use std::collections::HashMap;

/// An Instance that has been resolved
pub struct ResolvedInstance<'a, T: TimeRep, W: WidthRep> {
    sig: &'a core::Signature<T, W>,
    binds: Vec<W>,
}

impl<'a, T: TimeRep, W: WidthRep> ResolvedInstance<'a, T, W> {
    pub fn bound(sig: &'a core::Signature<T, W>, binds: Vec<W>) -> Self {
        log::trace!("sig = {}, binds = {:?}", sig, binds);
        Self { sig, binds }
    }
}

impl<'a, T: TimeRep> ResolvedInstance<'a, T, core::PortParam> {
    /// Construct a binding for this component instance
    pub fn this(sig: &'a core::Signature<T, core::PortParam>) -> Self {
        let binds = sig
            .params
            .iter()
            .map(|p| core::PortParam::Var(p.clone()))
            .collect_vec();
        Self::bound(sig, binds)
    }
}

// impl<'a, T: TimeRep> ResolvedInstance<'a, T, u64> {
//     /// Construct a binding for this component instance
//     pub fn this(sig: &'a core::Signature<T, u64>) -> Self {
//         assert!(
//             sig.params.is_empty(),
//             "Cannot bind instance with parameters"
//         );
//         Self::bound(sig, vec![])
//     }
// }

impl<'a, T: TimeRep, W: WidthRep> ResolvedInstance<'a, T, W> {
    pub fn sig(&self) -> &'a core::Signature<T, W> {
        self.sig
    }

    // Return the abstract variables defined by the signature of this instance.
    pub fn events(&self) -> Vec<core::Id> {
        self.sig.events().collect()
    }

    pub fn abstract_vars(&self) -> &[core::EventBind<T>] {
        &self.sig.events
    }

    pub fn interface_signals(&self) -> &[core::InterfaceDef] {
        &self.sig.interface_signals
    }

    pub fn input_names(&self) -> Vec<core::Id> {
        self.sig.inputs().map(|pd| pd.name.clone()).collect()
    }

    pub fn interface_name(&self) -> Vec<core::Id> {
        self.sig
            .interface_signals
            .iter()
            .map(|id| id.name.clone())
            .collect()
    }

    pub fn get_interface(
        &self,
        event: &core::Id,
    ) -> Option<&core::InterfaceDef> {
        self.sig.get_interface(event)
    }

    pub fn get_event(&self, event: &core::Id) -> Option<&core::EventBind<T>> {
        Some(self.sig.get_event(event))
    }

    pub fn phantom_events(&self) -> Vec<core::Id> {
        self.sig.phantom_events().collect()
    }

    pub fn resolve(&self) -> FilamentResult<core::Signature<T, W>> {
        self.sig.resolve(&self.binds).map_err(|e| {
            e.add_note(
                "Attempting to resolve signature",
                self.sig.name.copy_span(),
            )
        })
    }

    pub fn binding(&self, abs: &'a [T]) -> core::Binding<T> {
        self.sig.binding(abs)
    }
}

/// Environment to store the current set of bindings
pub struct Bindings<'a, T: TimeRep, W: WidthRep> {
    /// Signatures for external definitions
    ext_sigs: HashMap<core::Id, &'a core::Signature<T, W>>,
    /// Signatures for components
    comps: Vec<core::Component<T, W>>,
}
impl<'a, T: TimeRep, W: WidthRep> Bindings<'a, T, W> {
    pub fn new(
        ext_sigs: impl IntoIterator<Item = (core::Id, &'a core::Signature<T, W>)>,
    ) -> Self {
        Self {
            ext_sigs: ext_sigs.into_iter().collect(),
            comps: Vec::new(),
        }
    }

    /// Add a component definition to the environment
    pub fn add_component(&mut self, comp: core::Component<T, W>) {
        self.comps.push(comp);
    }

    /// Get a binding associated with a name or return error
    pub fn find_component(
        &'a self,
        name: &core::Id,
    ) -> FilamentResult<&'a core::Signature<T, W>> {
        if let Some(sig) = self.ext_sigs.get(name) {
            Ok(sig)
        } else if let Some(comp) =
            self.comps.iter().find(|c| c.sig.name == name)
        {
            Ok(&comp.sig)
        } else {
            Err(Error::undefined(name.clone(), "component")
                .add_note("Undefined component", name.copy_span()))
        }
    }

    // Returns a component or panics
    pub fn get_component(
        &'a self,
        name: &core::Id,
    ) -> &'a core::Signature<T, W> {
        self.find_component(name)
            .unwrap_or_else(|_| panic!("Failed to find component {}", name))
    }
}
impl<T: TimeRep, W: WidthRep> From<Bindings<'_, T, W>>
    for Vec<core::Component<T, W>>
{
    fn from(bind: Bindings<'_, T, W>) -> Self {
        bind.comps
    }
}
