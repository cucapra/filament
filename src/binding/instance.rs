use super::{CompBinding, InvIdx, SigIdx};
use crate::core;
use crate::{errors::WithPos, utils::GPosIdx};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an instance bound in a component
/// Defined methods represent operations on an instance and require a
/// component binding to be resolved.
pub struct InstIdx(pub(super) usize);

impl InstIdx {
    /// The Unknown instance
    pub const UNKNOWN: InstIdx = InstIdx(usize::MAX);
}

impl InstIdx {
    /// Get the position of the instance
    pub fn pos(&self, ctx: &CompBinding) -> GPosIdx {
        ctx[*self].pos
    }

    /// Returns all the invocations associated with an instance
    pub fn get_all_invokes<'a>(
        &'a self,
        ctx: &'a CompBinding<'a>,
    ) -> impl Iterator<Item = InvIdx> + '_ {
        ctx.invocations().filter(|inv| ctx[*inv].instance == *self)
    }

    /// Get the signature of this instance by resolving against the parameter bindings.
    /// Note that such a signature still has unresolved event bindings (such as the delay of a Register)
    /// that are only resolved through an invocation.
    pub(super) fn param_resolved_signature(
        &self,
        ctx: &CompBinding,
    ) -> core::Signature {
        let inst = &ctx[*self];
        ctx.prog.sig(inst.sig).resolve_offset(&inst.params)
    }
}

/// An instance bound by a component
pub struct BoundInstance {
    /// The signature of this instance
    pub sig: SigIdx,
    /// Parameter binding for this instance
    pub params: Vec<core::Expr>,
    /// Position associated with this instance
    pos: GPosIdx,
}

impl BoundInstance {
    /// Create a new instance
    pub fn new(sig: SigIdx, params: Vec<core::Expr>, pos: GPosIdx) -> Self {
        Self { sig, params, pos }
    }
}

impl WithPos for BoundInstance {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}
