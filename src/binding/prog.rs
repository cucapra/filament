use itertools::Itertools;

use crate::{
    core::{self, Id, Time},
    utils,
};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of a signature bound in the program
pub enum SigIdx {
    /// An external component
    Ext(usize),
    /// A Filament component
    Comp(usize),
}

impl SigIdx {
    /// The Unknown signature
    pub const UNKNOWN: SigIdx = SigIdx::Ext(usize::MAX);
}

/// Signatures bound in a program.
/// Also acts a dispatcher for methods on [[core::Signature]] since external and
/// component signatures have different types.
pub struct ProgBinding<'a> {
    externals: Vec<&'a core::Signature>,
    components: Vec<&'a core::Signature>,
    name_map: HashMap<Id, SigIdx>,
}

impl<'a> ProgBinding<'a> {
    /// Get index associated with a signature
    pub(super) fn find_sig_idx(&self, name: &core::Id) -> Option<SigIdx> {
        if let Some(idx) = self.externals.iter().position(|s| *name == s.name) {
            Some(SigIdx::Ext(idx))
        } else {
            self.components
                .iter()
                .position(|s| *name == s.name)
                .map(SigIdx::Comp)
        }
    }

    /// Get the signature index associated with a name.
    /// Panic if the signature is not found.
    pub(super) fn get_sig_idx(&self, name: &core::Id) -> SigIdx {
        self.find_sig_idx(name)
            .unwrap_or_else(|| panic!("Unknown signature: {}", name))
    }

    /// Event binding generated from a signature
    /// XXX: Can be constructed using the binding and the event names
    pub(super) fn event_binding(
        &self,
        sig: SigIdx,
        events: &[Time],
    ) -> utils::Binding<Time> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx].event_binding(events),
            SigIdx::Comp(idx) => self.components[idx].event_binding(events),
        }
    }

    /// XXX: Can be constructed using the binding and the param names
    pub(super) fn param_binding(
        &self,
        sig: SigIdx,
        params: &[core::Expr],
    ) -> utils::Binding<core::Expr> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx].param_binding(params),
            SigIdx::Comp(idx) => self.components[idx].param_binding(params),
        }
    }

    /// Add a component signature to the program binding
    pub fn add_component(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::Comp(self.components.len());
        self.components.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Add a component signature to the program binding
    pub fn add_external(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::Ext(self.externals.len());
        self.externals.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    // ============= Dispatch methods on Signatures =============

    /// Return the underlying signature
    pub fn sig(&self, sig: SigIdx) -> &'a core::Signature {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx],
            SigIdx::Comp(idx) => self.components[idx],
        }
    }

    /// Apply function on either an external or component signature and return the result
    #[inline]
    pub fn map_signature<O>(
        &self,
        sig: SigIdx,
        ext: impl Fn(&'a core::Signature) -> O,
        comp: impl Fn(&'a core::Signature) -> O,
    ) -> O {
        match sig {
            SigIdx::Ext(idx) => ext(self.externals[idx]),
            SigIdx::Comp(idx) => comp(self.components[idx]),
        }
    }

    /// Get all the outputs in a signature
    pub fn output_names(&self, sig: SigIdx) -> Vec<&Id> {
        self.map_signature(
            sig,
            |ext| ext.outputs().map(|pd| &pd.name).collect_vec(),
            |comp| comp.outputs().map(|pd| &pd.name).collect_vec(),
        )
    }

    /// Get all the inputs in a signature
    pub fn input_names(&self, sig: SigIdx) -> Vec<&Id> {
        self.map_signature(
            sig,
            |ext| ext.inputs().map(|pd| &pd.name).collect_vec(),
            |comp| comp.inputs().map(|pd| &pd.name).collect_vec(),
        )
    }

    /// Name of the events associated with a signature
    pub fn event_names(&self, sig: SigIdx) -> Vec<&Id> {
        self.map_signature(
            sig,
            |ext| ext.events.iter().map(|eb| &eb.event).collect_vec(),
            |comp| comp.events.iter().map(|eb| &eb.event).collect_vec(),
        )
    }

    /// Name of the parameters associated with a signature
    pub fn param_names(&self, sig: SigIdx) -> &Vec<Id> {
        self.map_signature(sig, |ext| &ext.params, |comp| &comp.params)
    }

    /// Get the phantom events
    pub fn phantom_events(&self, sig: SigIdx) -> Vec<Id> {
        self.map_signature(
            sig,
            |ext| ext.phantom_events().collect_vec(),
            |comp| comp.phantom_events().collect_vec(),
        )
    }

    /// Returns the underlying comp signature. Panics if the signature actually points to an external.
    pub fn comp_sig(&self, sig: SigIdx) -> &'a core::Signature {
        match sig {
            SigIdx::Ext(_) => {
                unreachable!("comp_sig called on external signature")
            }
            SigIdx::Comp(idx) => self.components[idx],
        }
    }
}

impl<'a> From<&'a core::Namespace> for ProgBinding<'a> {
    fn from(ns: &'a core::Namespace) -> Self {
        let mut ctx = ProgBinding {
            externals: Vec::new(),
            components: Vec::new(),
            name_map: HashMap::new(),
        };
        ns.externs
            .iter()
            .flat_map(|(_, comps)| comps)
            .for_each(|c| {
                ctx.add_external(c);
            });
        ns.components.iter().map(|comp| &comp.sig).for_each(|c| {
            ctx.add_component(c);
        });
        ctx
    }
}
