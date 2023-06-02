use super::{Component, Ctx, ExprIdx, Range};
use crate::{ast, utils::GPosIdx};
use codespan_reporting::diagnostic::Diagnostic;

#[derive(Default)]
/// Information associated with the IR.
pub enum Info {
    #[default]
    /// An absence of information is still information
    Empty,
    /// Assertion information
    Assert(Reason),
    /// For [super::Param]
    Param {
        /// Surface-level name of the parameter
        name: ast::Id,
        bind_loc: GPosIdx,
    },
    /// For [super::Event]
    Event {
        /// Surface-level name of the event
        name: ast::Id,
        bind_loc: GPosIdx,
        delay_loc: GPosIdx,
    },
    /// For [super::Instance]
    Instance {
        name_loc: GPosIdx,
        comp_loc: GPosIdx,
        bind_loc: GPosIdx,
    },
    /// For [super::Invoke]
    Invoke {
        name_loc: GPosIdx,
        inst_loc: GPosIdx,
        bind_loc: GPosIdx,
    },
    /// For [super::Connect]
    Connect { dst_loc: GPosIdx, src_loc: GPosIdx },
    /// For [super::Port]
    Port {
        /// Surface-level name
        name: ast::Id,
        bind_loc: GPosIdx,
        width_loc: GPosIdx,
        live_loc: GPosIdx,
    },
}

impl Info {
    pub fn assert(reason: Reason) -> Self {
        Self::Assert(reason)
    }

    pub fn param(name: ast::Id, bind_loc: GPosIdx) -> Self {
        Self::Param { name, bind_loc }
    }

    pub fn event(name: ast::Id, bind_loc: GPosIdx, delay_loc: GPosIdx) -> Self {
        Self::Event {
            name,
            bind_loc,
            delay_loc,
        }
    }

    pub fn instance(
        name_loc: GPosIdx,
        comp_loc: GPosIdx,
        bind_loc: GPosIdx,
    ) -> Self {
        Self::Instance {
            name_loc,
            comp_loc,
            bind_loc,
        }
    }

    pub fn connect(dst_loc: GPosIdx, src_loc: GPosIdx) -> Self {
        Self::Connect { dst_loc, src_loc }
    }

    pub fn port(
        name: ast::Id,
        bind_loc: GPosIdx,
        width_loc: GPosIdx,
        live_loc: GPosIdx,
    ) -> Self {
        Self::Port {
            name,
            bind_loc,
            width_loc,
            live_loc,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
/// Why was an assertion created?
pub enum Reason {
    /// Assertion representing constraint on a parameter
    ParamConstraint {
        /// Location of parameter definition
        param_def_loc: GPosIdx,
        /// Location of the binding
        bind_loc: GPosIdx,
        /// Location of the constraint
        constraint_loc: GPosIdx,
    },
    /// Assertion representing constraint on an event
    EventConstraint {
        /// Location of event definition
        event_def_loc: GPosIdx,
        /// Location of the binding
        bind_loc: GPosIdx,
        /// Location of the constraint
        constraint_loc: GPosIdx,
    },
    /// Assertion requiring that the source is available for at least as long as
    /// the destination requires
    Liveness {
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_liveness: Range,
        src_liveness: Range,
    },
    /// Require that lengths of bundles match
    BundleLenMatch {
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_width: ExprIdx,
        src_width: ExprIdx,
    },
    /// An access is within bounds
    InBoundsAccess {
        // Defining location for the port
        def_loc: GPosIdx,
        /// Location of the access
        access_loc: GPosIdx,
        /// Length of the bundle
        bundle_len: ExprIdx,
    },
    /// A simple reason
    Misc { reason: String, def_loc: GPosIdx },
}

impl Reason {
    pub fn misc<S: ToString>(r: S, def_loc: GPosIdx) -> Self {
        Self::Misc {
            reason: r.to_string(),
            def_loc,
        }
    }

    pub fn liveness(
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_liveness: Range,
        src_liveness: Range,
    ) -> Self {
        Self::Liveness {
            dst_loc,
            src_loc,
            dst_liveness,
            src_liveness,
        }
    }

    pub fn param_cons(
        param_def_loc: GPosIdx,
        bind_loc: GPosIdx,
        constraint_loc: GPosIdx,
    ) -> Self {
        Self::ParamConstraint {
            param_def_loc,
            bind_loc,
            constraint_loc,
        }
    }

    pub fn event_cons(
        event_def_loc: GPosIdx,
        bind_loc: GPosIdx,
        constraint_loc: GPosIdx,
    ) -> Self {
        Self::EventConstraint {
            event_def_loc,
            bind_loc,
            constraint_loc,
        }
    }

    pub fn in_bounds_access(
        def_loc: GPosIdx,
        access_loc: GPosIdx,
        bundle_len: ExprIdx,
    ) -> Self {
        Self::InBoundsAccess {
            def_loc,
            access_loc,
            bundle_len,
        }
    }

    pub fn bundle_len_match(
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_width: ExprIdx,
        src_width: ExprIdx,
    ) -> Self {
        Self::BundleLenMatch {
            dst_loc,
            src_loc,
            dst_width,
            src_width,
        }
    }
}

impl From<Reason> for Info {
    fn from(r: Reason) -> Self {
        Self::Assert(r)
    }
}

impl Reason {
    /// Convert this reason into a diagnostic message
    pub fn diag(&self, ctx: &Component) -> Diagnostic<usize> {
        match self {
            Reason::Misc { reason, def_loc } => {
                let label = def_loc.primary().with_message(reason.clone());
                Diagnostic::error()
                    .with_message(reason)
                    .with_labels(vec![label])
            }
            Reason::InBoundsAccess {
                def_loc,
                access_loc,
                bundle_len,
            } => {
                let access =
                    access_loc.primary().with_message("out of bounds access");
                let def = def_loc.secondary().with_message(format!(
                    "bundle's length is {}",
                    ctx.display(*bundle_len)
                ));
                Diagnostic::error()
                    .with_message("out of bounds access of bundle")
                    .with_labels(vec![access, def])
            }
            Reason::BundleLenMatch {
                dst_loc,
                src_loc,
                dst_width,
                src_width,
            } => {
                let sw = ctx.display(*src_width);
                let dw = ctx.display(*dst_width);
                let src = src_loc
                    .secondary()
                    .with_message(format!("source's length is {sw}",));
                let dst = dst_loc
                    .primary()
                    .with_message(format!("destination's length is {dw}",));
                Diagnostic::error()
                    .with_message(format!("mismatched bundle lengths: required bundle of size `{dw}' but found bundle of size `{sw}'"))
                    .with_labels(vec![src, dst])
            }
            Reason::Liveness {
                dst_loc,
                src_loc,
                dst_liveness,
                src_liveness,
            } => {
                let sl = src_loc.primary().with_message(format!(
                    "source is available for {}",
                    src_liveness
                ));
                let dl = dst_loc.secondary().with_message(format!(
                    "requires value for {}",
                    dst_liveness
                ));
                Diagnostic::error()
                    .with_message("source port does not provide value for as long as destination requires")
                    .with_labels(vec![sl, dl])
            }
            _ => todo!(),
        }
    }
}
