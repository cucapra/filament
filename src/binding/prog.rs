use crate::{
    core::{self, Id},
    diagnostics, utils,
};
use std::collections::HashMap;

use super::BoundComponent;

pub type SigIdx = utils::Idx<core::Signature>;

/// Signatures bound in a program.
/// Also acts a dispatcher for methods on [core::Signature] since external and
/// component signatures have different types.
pub struct ProgBinding<'a> {
    signatures: Vec<&'a core::Signature>,
    /// Component bindings. Invariant: the index of the component in this vector is the same as
    /// the index of the component signature in the `signatures` vector.
    comps: Vec<super::BoundComponent>,
    name_map: HashMap<Id, SigIdx>,
}

impl<'a> ProgBinding<'a> {
    /// Add a component signature to the program binding and build the
    /// [super::BoundComponent] struct for it.
    fn add_comp_sig(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::new(self.signatures.len());
        self.signatures.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Build a [BoundComponent] for the component and return a boolean indicating if there were
    /// any errors.
    fn add_comp_bind(
        &mut self,
        comp: &'a core::Component,
        diag: &mut diagnostics::Diagnostics,
    ) {
        let idx = self.get_sig_idx(&comp.sig.name);
        let loc = self.comps.len();
        debug_assert!(
            idx.get() == loc,
            "Component body added to a different index than signature"
        );

        let mut bind = BoundComponent::from(idx);
        bind.process_checked_cmds(self, &comp.body, diag);
        self.comps.push(bind);
    }

    /// Add a component signature to the program binding
    fn add_external(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::new(self.signatures.len());
        self.signatures.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Get the signature index associated with a name.
    /// Panic if the signature is not found.
    pub(super) fn get_sig_idx(&self, name: &core::Id) -> SigIdx {
        self.find_sig_idx(name)
            .unwrap_or_else(|| panic!("Unknown signature: {}", name))
    }

    /// Get index associated with a signature
    pub(super) fn find_sig_idx(&self, name: &core::Id) -> Option<SigIdx> {
        self.name_map.get(name).copied()
    }
}

impl<'a> TryFrom<&'a core::Namespace> for ProgBinding<'a> {
    type Error = u64;

    fn try_from(ns: &'a core::Namespace) -> Result<Self, Self::Error> {
        let mut ctx = ProgBinding {
            signatures: Vec::new(),
            comps: Vec::with_capacity(ns.components.len()),
            name_map: HashMap::new(),
        };
        // Add component signatures first so that we can index comps vector using the same
        // index as the signature.
        ns.components.iter().for_each(|c| {
            ctx.add_comp_sig(&c.sig);
        });
        ns.externs
            .iter()
            .flat_map(|(_, comps)| comps)
            .for_each(|c| {
                ctx.add_external(c);
            });

        let mut diag = diagnostics::Diagnostics::default();
        // Build and add bindings for components
        ns.components.iter().for_each(|c| {
            ctx.add_comp_bind(c, &mut diag);
        });
        if let Some(errs) = diag.report_all() {
            Err(errs)
        } else {
            Ok(ctx)
        }
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

impl std::ops::Index<&Id> for ProgBinding<'_> {
    type Output = core::Signature;
    fn index(&self, name: &Id) -> &Self::Output {
        let idx = self.get_sig_idx(name);
        &self[idx]
    }
}
