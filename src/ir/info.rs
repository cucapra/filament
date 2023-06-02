use super::{ExprIdx, Range};
use crate::{
    ast,
    utils::{GPosIdx, GlobalPositionTable},
};
use codespan_reporting::diagnostic::{Diagnostic, Label, LabelStyle};

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
    /// Require that bitwidths of ports match
    BitwidthMatch {
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_width: ExprIdx,
        src_width: ExprIdx,
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
}

impl From<Reason> for Info {
    fn from(r: Reason) -> Self {
        Self::Assert(r)
    }
}

impl<'a> From<&'a Reason> for Diagnostic<usize> {
    fn from(r: &'a Reason) -> Self {
        let table = GlobalPositionTable::as_ref();
        match r {
            Reason::Misc { reason, def_loc } => {
                let pos = table.get_pos(def_loc.0);
                let label = Label::new(
                    LabelStyle::Primary,
                    pos.file.get(),
                    pos.start..pos.end,
                )
                .with_message(reason.clone());
                Diagnostic::error()
                    .with_message(reason)
                    .with_labels(vec![label])
            }
            _ => todo!(),
        }
    }
}
