use itertools::Itertools;

use crate::core::{self, TimeRep, WidthRep, WithTime};
use crate::errors::WithPos;
use crate::errors::{Error, FilamentResult};
use std::collections::HashMap;

pub enum ResolvedInstance<'a, T: TimeRep, W: WidthRep> {
    /// A component that uses width `W`
    Component {
        sig: &'a core::Signature<T, W>,
        binds: Vec<W>,
    },
    /// An external that always uses `PortParam`
    External {
        sig: &'a core::Signature<T, core::PortParam>,
        binds: Vec<W>,
    },
}

impl<'a, T: TimeRep, W: WidthRep> ResolvedInstance<'a, T, W> {
    pub fn bound(sig: &'a core::Signature<T, W>, binds: Vec<W>) -> Self {
        log::trace!("sig = {}, binds = {:?}", sig, binds);
        Self::Component { sig, binds }
    }

    pub fn external(
        sig: &'a core::Signature<T, core::PortParam>,
        binds: Vec<W>,
    ) -> Self {
        log::trace!("sig = {}, binds = {:?}", sig, binds);
        Self::External { sig, binds }
    }

    /// Construct a binding for this component instance
    pub fn this(sig: &'a core::Signature<T, W>) -> Self {
        let binds =
            sig.params.iter().map(|p| W::param(p.clone())).collect_vec();
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
    pub fn params(&self) -> &[core::Id] {
        match self {
            Self::Component { sig, .. } => &sig.params,
            Self::External { sig, .. } => &sig.params,
        }
    }

    // Return the abstract variables defined by the signature of this instance.
    pub fn events(&self) -> Vec<core::Id> {
        match self {
            Self::Component { sig, .. } => sig.events().collect(),
            Self::External { sig, .. } => sig.events().collect(),
        }
    }

    pub fn abstract_vars(&self) -> &[core::EventBind<T>] {
        match self {
            Self::Component { sig, .. } => &sig.events,
            Self::External { sig, .. } => &sig.events,
        }
    }

    pub fn interface_signals(&self) -> &[core::InterfaceDef] {
        match self {
            Self::Component { sig, .. } => &sig.interface_signals,
            Self::External { sig, .. } => &sig.interface_signals,
        }
    }

    pub fn input_names(&self) -> Vec<core::Id> {
        match self {
            Self::Component { sig, .. } => {
                sig.inputs().map(|pd| pd.name.clone()).collect()
            }
            Self::External { sig, .. } => {
                sig.inputs().map(|pd| pd.name.clone()).collect()
            }
        }
    }

    pub fn interface_name(&self) -> Vec<core::Id> {
        let interfaces = match self {
            Self::Component { sig, .. } => &sig.interface_signals,
            Self::External { sig, .. } => &sig.interface_signals,
        };
        interfaces.iter().map(|id| id.name.clone()).collect()
    }

    pub fn get_interface(
        &self,
        event: &core::Id,
    ) -> Option<&core::InterfaceDef> {
        match self {
            Self::Component { sig, .. } => sig.get_interface(event),
            Self::External { sig, .. } => sig.get_interface(event),
        }
    }

    pub fn get_event(&self, event: &core::Id) -> Option<&core::EventBind<T>> {
        Some(match self {
            Self::Component { sig, .. } => sig.get_event(event),
            Self::External { sig, .. } => sig.get_event(event),
        })
    }

    pub fn phantom_events(&self) -> Vec<core::Id> {
        match self {
            Self::Component { sig, .. } => sig.phantom_events().collect(),
            Self::External { sig, .. } => sig.phantom_events().collect(),
        }
    }

    pub fn resolve(&self) -> FilamentResult<core::Signature<T, W>> {
        match self {
            Self::Component { sig, binds } => sig.resolve(binds).map_err(|e| {
                e.add_note(
                    "Attempting to resolve signature",
                    sig.name.copy_span(),
                )
            }),
            Self::External { sig, binds } => sig.resolve(binds).map_err(|e| {
                e.add_note(
                    "Attempting to resolve signature",
                    sig.name.copy_span(),
                )
            }),
        }
    }

    /// Event binding for this instance
    pub fn event_binding(&self, abs: &'a [T]) -> core::Binding<T> {
        match self {
            Self::Component { sig, .. } => sig.event_binding(abs),
            Self::External { sig, .. } => sig.event_binding(abs),
        }
    }

    pub fn param_binding(&self) -> core::Binding<W> {
        match self {
            Self::Component { sig, binds } => sig.param_binding(binds),
            Self::External { sig, binds } => sig.param_binding(binds),
        }
    }
}

/// Environment to store the current set of bindings
pub struct Bindings<'a, T: TimeRep, W: WidthRep> {
    /// Signatures for external definitions
    ext_sigs: HashMap<core::Id, &'a core::Signature<T, core::PortParam>>,
    /// Signatures for components
    comps: Vec<core::Component<T, W>>,
}
impl<'a, T: TimeRep, W: WidthRep> Bindings<'a, T, W> {
    pub fn new(
        ext_sigs: impl IntoIterator<
            Item = (core::Id, &'a core::Signature<T, core::PortParam>),
        >,
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
        binds: &[W],
    ) -> FilamentResult<ResolvedInstance<'a, T, W>> {
        if let Some(sig) = self.ext_sigs.get(name) {
            Ok(ResolvedInstance::external(sig, binds.to_vec()))
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
        name: &core::Id,
        binds: &[W],
    ) -> ResolvedInstance<'a, T, W> {
        self.find_component(name, binds)
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
