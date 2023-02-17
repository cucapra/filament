use crate::{
    core::{self, Id, Time},
    utils,
};
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of a signature bound in the program
pub enum SigIdx {
    /// An external component
    Ext(u32),
    /// A Filament component
    Comp(u32),
}

impl SigIdx {
    /// The Unknown signature
    pub const UNKNOWN: SigIdx = SigIdx::Ext(u32::MAX);
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
    fn ext(s: usize) -> SigIdx {
        SigIdx::Ext(s as u32)
    }

    fn comp(s: usize) -> SigIdx {
        SigIdx::Comp(s as u32)
    }

    /// Get index associated with a signature
    pub(super) fn find_sig_idx(&self, name: &core::Id) -> Option<SigIdx> {
        if let Some(idx) = self.externals.iter().position(|s| *name == s.name) {
            Some(Self::ext(idx))
        } else {
            self.components
                .iter()
                .position(|s| *name == s.name)
                .map(Self::comp)
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
            SigIdx::Ext(idx) => {
                self.externals[idx as usize].event_binding(events)
            }
            SigIdx::Comp(idx) => {
                self.components[idx as usize].event_binding(events)
            }
        }
    }

    /// XXX: Can be constructed using the binding and the param names
    pub(super) fn param_binding(
        &self,
        sig: SigIdx,
        params: &[core::Expr],
    ) -> utils::Binding<core::Expr> {
        match sig {
            SigIdx::Ext(idx) => {
                self.externals[idx as usize].param_binding(params)
            }
            SigIdx::Comp(idx) => {
                self.components[idx as usize].param_binding(params)
            }
        }
    }

    /// Add a component signature to the program binding
    fn add_component(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = Self::comp(self.components.len());
        self.components.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Add a component signature to the program binding
    fn add_external(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = Self::ext(self.externals.len());
        self.externals.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
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

impl std::ops::Index<SigIdx> for ProgBinding<'_> {
    type Output = core::Signature;
    fn index(&self, idx: SigIdx) -> &Self::Output {
        debug_assert!(
            idx != SigIdx::UNKNOWN,
            "Attempted to use unknown signature"
        );
        match idx {
            SigIdx::Ext(idx) => self.externals[idx as usize],
            SigIdx::Comp(idx) => self.components[idx as usize],
        }
    }
}
