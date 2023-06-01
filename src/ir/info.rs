use super::Range;
use crate::{ast, utils::GPosIdx};

#[derive(Default)]
/// Information associated with the IR.
pub enum Info {
    #[default]
    /// An absence of information is still information
    Empty,
    /// Assertion information
    Assert(AssertReason),
    /// Parameter information
    Param {
        /// Surface-level name of the parameter
        name: ast::Id,
        bind_loc: GPosIdx,
    },
    /// Event Information
    Event {
        /// Surface-level name of the event
        name: ast::Id,
        bind_loc: GPosIdx,
        delay_loc: GPosIdx,
    },
    /// Information for an instance
    Instance {
        name_loc: GPosIdx,
        comp_loc: GPosIdx,
        bind_loc: GPosIdx,
    },
    /// Information for an invocation
    Invoke {
        name_loc: GPosIdx,
        inst_loc: GPosIdx,
        bind_loc: GPosIdx,
    },
    /// Information for Connection
    Connect { dst_loc: GPosIdx, src_loc: GPosIdx },
}

impl Info {
    /// Construct information about a parameter
    pub fn param(name: ast::Id, bind_loc: GPosIdx) -> Self {
        Self::Param { name, bind_loc }
    }

    /// Construct information about an event
    pub fn event(name: ast::Id, bind_loc: GPosIdx, delay_loc: GPosIdx) -> Self {
        Self::Event {
            name,
            bind_loc,
            delay_loc,
        }
    }

    /// Construct information about an instance
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

    /// Construct information about an invocation
    pub fn connect(dst_loc: GPosIdx, src_loc: GPosIdx) -> Self {
        Self::Connect { dst_loc, src_loc }
    }
}

/// Why was an assertion created?
pub enum AssertReason {
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
}
