use crate::{
    ast::{self, Id},
    diagnostics,
    errors::Error,
    idx,
};
use std::collections::HashMap;

use super::BoundComponent;

pub type SigIdx = idx!(ast::Signature);

/// Signatures bound in a program.
/// Also acts a dispatcher for methods on [ast::Signature] since external and
/// component signatures have different types.
pub struct ProgBinding<'a> {
    signatures: Vec<&'a ast::Signature>,
    /// Component bindings. Invariant: the index of the component in this vector is the same as
    /// the index of the component signature in the `signatures` vector.
    comps: Vec<super::BoundComponent>,
    name_map: HashMap<Id, SigIdx>,
}

impl<'a> ProgBinding<'a> {
    // Is this name already bound?
    fn is_bound(&self, name: &ast::Id) -> Option<&ast::Signature> {
        self.name_map
            .get(name)
            .map(|idx| self.signatures[idx.get()])
    }

    /// Build a [BoundComponent] for the component and return a boolean indicating if there were
    /// any errors.
    fn add_comp_bind(
        &mut self,
        comp: &'a ast::Component,
        diag: &mut diagnostics::Diagnostics,
    ) {
        let idx = self.get_sig_idx(&comp.sig.name);
        let loc = self.comps.len();
        debug_assert!(
            idx.get() == loc,
            "Component binding added to a different index ({}) than its signature ({loc})",
            idx.get(),
        );

        let mut bind = BoundComponent::from(idx);
        // Add all the bundles from the signature
        for pd in comp.sig.ports() {
            if let ast::PortDef::Bundle(bun) = pd.inner() {
                bind.add_bundle(bun.clone());
            }
        }
        bind.process_checked_cmds(self, &comp.body, diag);
        self.comps.push(bind);
    }

    /// Add a component signature to the program binding
    fn add_sig(
        &mut self,
        sig: &'a ast::Signature,
        diag: &mut diagnostics::Diagnostics,
    ) -> SigIdx {
        if let Some(old_sig) = self.is_bound(&sig.name) {
            let err = Error::already_bound(*sig.name.inner(), "signature")
                .add_note(diag.add_info(
                    "signature with same name is already bound",
                    sig.name.pos(),
                ))
                .add_note(
                    diag.add_info("previous definition", old_sig.name.pos()),
                );
            diag.add_error(err);
            self.get_sig_idx(&sig.name)
        } else {
            let idx = SigIdx::new(self.signatures.len());
            self.signatures.push(sig);
            self.name_map.insert(*sig.name.inner(), idx);
            idx
        }
    }

    pub(super) fn get_comp_binding(&self, name: &ast::Id) -> &BoundComponent {
        let idx = self.get_sig_idx(name);
        &self.comps[idx.get()]
    }

    /// Get the signature index associated with a name.
    /// Panic if the signature is not found.
    pub(super) fn get_sig_idx(&self, name: &ast::Id) -> SigIdx {
        self.find_sig_idx(name)
            .unwrap_or_else(|| panic!("Unknown signature: {}", name))
    }

    /// Get index associated with a signature
    pub(super) fn find_sig_idx(&self, name: &ast::Id) -> Option<SigIdx> {
        self.name_map.get(name).copied()
    }
}

impl<'a> TryFrom<&'a ast::Namespace> for ProgBinding<'a> {
    type Error = u64;

    fn try_from(ns: &'a ast::Namespace) -> Result<Self, Self::Error> {
        let mut ctx = ProgBinding {
            signatures: Vec::new(),
            comps: Vec::with_capacity(ns.components.len()),
            name_map: HashMap::new(),
        };
        let mut diag = diagnostics::Diagnostics::default();
        // Add component signatures first so that we can index comps vector using the same
        // index as the signature.
        ns.components.iter().for_each(|c| {
            ctx.add_sig(&c.sig, &mut diag);
        });
        ns.externs
            .iter()
            .flat_map(|(_, comps)| comps)
            .for_each(|c| {
                ctx.add_sig(c, &mut diag);
            });

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
    type Output = ast::Signature;
    fn index(&self, idx: SigIdx) -> &Self::Output {
        self.signatures[idx.get()]
    }
}

impl std::ops::Index<&Id> for ProgBinding<'_> {
    type Output = ast::Signature;
    fn index(&self, name: &Id) -> &Self::Output {
        let idx = self.get_sig_idx(name);
        &self[idx]
    }
}
