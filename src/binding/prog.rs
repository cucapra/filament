use crate::{
    core::{self, Id},
    utils,
};
use std::collections::HashMap;

pub type SigIdx = utils::Idx<core::Signature>;

/// Signatures bound in a program.
/// Also acts a dispatcher for methods on [[core::Signature]] since external and
/// component signatures have different types.
pub struct ProgBinding<'a> {
    signatures: Vec<&'a core::Signature>,
    name_map: HashMap<Id, SigIdx>,
}

impl<'a> ProgBinding<'a> {
    /// Add a component signature to the program binding
    fn add_component(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::new(self.signatures.len());
        self.signatures.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Add a component signature to the program binding
    fn add_external(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::new(self.signatures.len());
        self.signatures.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Get index associated with a signature
    pub fn find_sig_idx(&self, name: &core::Id) -> Option<SigIdx> {
        if let Some(idx) = self.signatures.iter().position(|s| *name == s.name)
        {
            Some(SigIdx::new(idx))
        } else {
            self.signatures
                .iter()
                .position(|s| *name == s.name)
                .map(SigIdx::new)
        }
    }

    /// Get the signature index associated with a name.
    /// Panic if the signature is not found.
    pub fn get_sig_idx(&self, name: &core::Id) -> SigIdx {
        self.find_sig_idx(name)
            .unwrap_or_else(|| panic!("Unknown signature: {}", name))
    }
}

impl<'a> From<&'a core::Namespace> for ProgBinding<'a> {
    fn from(ns: &'a core::Namespace) -> Self {
        let mut ctx = ProgBinding {
            signatures: Vec::new(),
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
        self.signatures[idx.get()]
    }
}
