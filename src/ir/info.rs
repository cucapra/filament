use super::{Component, Ctx, ExprIdx, Range, TimeIdx, TimeSub};
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

    // ============ Constraints from type checking ==============
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

    // ========== Constraints from interval checking ============
    /// Assertion requiring that the source is available for at least as long as
    /// the destination requires
    Liveness {
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_liveness: Range,
        src_liveness: Range,
    },
    /// Bundle's delay must be less than the event's
    BundleDelay {
        event_delay_loc: GPosIdx,
        bundle_range_loc: GPosIdx,
        bundle_live: TimeSub,
        /// The bundle paramemter's binding location
        param_loc: GPosIdx,
        /// The start and end of index's range
        param_range: (ExprIdx, ExprIdx),
    },
    /// Well formed time interval
    WellFormedInterval {
        /// The range's location
        range_loc: GPosIdx,
        /// The range's start and end
        range: (TimeIdx, TimeIdx),
    },
    // =============== Generic Constraints =======================
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

    pub fn bundle_delay(
        event_delay_loc: GPosIdx,
        bundle_range_loc: GPosIdx,
        bundle_live: TimeSub,
        param_loc: GPosIdx,
        param_range: (ExprIdx, ExprIdx),
    ) -> Self {
        Self::BundleDelay {
            event_delay_loc,
            bundle_range_loc,
            bundle_live,
            param_loc,
            param_range,
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

    pub fn well_formed_interval(
        range_loc: GPosIdx,
        range: (TimeIdx, TimeIdx),
    ) -> Self {
        Self::WellFormedInterval { range_loc, range }
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
                    .with_message(format!("length of bundle is {sw}",));
                let dst = dst_loc
                    .primary()
                    .with_message(format!("length of bunle is {dw}",));
                Diagnostic::error()
                    .with_message(format!("required bundle of size `{dw}' but found bundle of size `{sw}'"))
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
                    ctx.display_range(src_liveness)
                ));
                let dl = dst_loc.secondary().with_message(format!(
                    "requires value for {}",
                    ctx.display_range(dst_liveness)
                ));
                Diagnostic::error()
                    .with_message("source port does not provide value for as long as destination requires")
                    .with_labels(vec![sl, dl])
            }
            Reason::BundleDelay {
                event_delay_loc,
                bundle_range_loc,
                bundle_live,
                param_loc,
                param_range,
            } => {
                let wire = bundle_range_loc.primary().with_message(format!(
                    "available for {}",
                    ctx.display_timesub(bundle_live)
                ));
                let event =
                    event_delay_loc.secondary().with_message("event's delay");
                let mut labels = vec![wire, event];

                // If the parameter location is not defined, we do not report its location
                if let Some(loc) = param_loc.into_option() {
                    let param = loc.secondary().with_message(format!(
                        "takes values in [{}, {})",
                        ctx.display(param_range.0),
                        ctx.display(param_range.1)
                    ));
                    labels.push(param);
                }

                Diagnostic::error()
                    .with_message("bundle's availability is greater than the delay of the event")
                    .with_labels(labels)
            }
            Reason::WellFormedInterval {
                range_loc,
                range: (start, end),
            } => {
                let label = range_loc
                    .primary()
                    .with_message(format!("interval's end `{}' is not strictly greater than the start `{}", ctx.display(*end), ctx.display(*start)));
                Diagnostic::error()
                    .with_message(
                        "interval's end must be strictly greater than the start",
                    )
                    .with_labels(vec![label])
            }
            _ => todo!(),
        }
    }
}
