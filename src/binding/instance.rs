use super::{CompBinding, InvIdx, SigIdx};
use crate::utils::GPosIdx;
use crate::{core, idx};
use itertools::Itertools;

pub type InstIdx = idx!(BoundInstance);

impl InstIdx {
    /// Get the position of the instance
    pub fn pos(&self, ctx: &CompBinding) -> GPosIdx {
        ctx[*self].pos
    }

    /// Returns all the invocations associated with an instance
    pub fn get_all_invokes<'a>(
        &'a self,
        ctx: &'a CompBinding<'_, 'a>,
    ) -> impl Iterator<Item = InvIdx> + '_ {
        ctx.invocations().filter(|inv| ctx[*inv].instance == *self)
    }

    /// Get the signature of this instance by resolving against the parameter bindings.
    /// Note that such a signature still has unresolved event bindings (such as the delay of a Register)
    /// that are only resolved through an invocation.
    pub fn param_resolved_signature(
        &self,
        ctx: &CompBinding,
    ) -> core::Signature {
        let inst = &ctx[*self];
        let binds: Vec<core::Expr> =
            inst.params.iter().map(|p| p.clone().take()).collect_vec();
        ctx.prog[inst.sig].clone().resolve_exprs(binds)
    }
}

/// An instance bound by a component
pub struct BoundInstance {
    /// The signature of this instance
    pub sig: SigIdx,
    /// Parameter binding for this instance
    pub params: Vec<core::Loc<core::Expr>>,
    /// Position associated with this instance
    pub(super) pos: GPosIdx,
}

impl BoundInstance {
    /// Create a new instance
    pub fn new(
        sig: SigIdx,
        params: Vec<core::Loc<core::Expr>>,
        pos: GPosIdx,
    ) -> Self {
        Self { sig, params, pos }
    }
}
