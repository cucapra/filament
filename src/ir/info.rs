use super::{Component, DisplayCtx, ExprIdx, Range, TimeIdx, TimeSub};
use crate::{ast, utils::GPosIdx};
use codespan_reporting::diagnostic::Diagnostic;
use struct_variant::struct_variant;

pub struct Empty;

pub struct Assert(pub Reason);

pub struct Param {
    pub name: ast::Id,
    pub bind_loc: GPosIdx,
}

pub struct Event {
    pub name: ast::Id,
    pub bind_loc: GPosIdx,
    pub delay_loc: GPosIdx,
    pub interface_name: Option<ast::Id>,
    pub interface_bind_loc: Option<GPosIdx>,
}

pub struct EventBind {
    pub ev_delay_loc: GPosIdx,
    pub bind_loc: GPosIdx,
}

pub struct Instance {
    pub name: ast::Id,
    pub comp_loc: GPosIdx,
    pub bind_loc: GPosIdx,
}

pub struct Invoke {
    pub name: ast::Id,
    pub inst_loc: GPosIdx,
    pub bind_loc: GPosIdx,
}

pub struct Connect {
    pub dst_loc: GPosIdx,
    pub src_loc: GPosIdx,
}

pub struct Port {
    pub name: ast::Id,
    pub bind_loc: GPosIdx,
    pub width_loc: GPosIdx,
    pub live_loc: GPosIdx,
}

/// Information associated with the IR.
#[struct_variant()]
pub enum Info {
    Empty,
    Assert,
    Param,
    Event,
    EventBind,
    Instance,
    Invoke,
    Connect,
    Port,
}

impl Info {
    pub fn empty() -> Self {
        Self::Empty(Empty)
    }

    pub fn assert(reason: Reason) -> Self {
        Assert(reason).into()
    }

    pub fn param(name: ast::Id, bind_loc: GPosIdx) -> Self {
        Param { name, bind_loc }.into()
    }

    pub fn event(
        name: ast::Id,
        bind_loc: GPosIdx,
        delay_loc: GPosIdx,
        interface_port: Option<(ast::Id, GPosIdx)>,
    ) -> Self {
        Event {
            name,
            bind_loc,
            delay_loc,
            interface_name: interface_port.map(|(n, _)| n),
            interface_bind_loc: interface_port.map(|(_, l)| l),
        }
        .into()
    }

    pub fn event_bind(ev_delay_loc: GPosIdx, bind_loc: GPosIdx) -> Self {
        EventBind {
            ev_delay_loc,
            bind_loc,
        }
        .into()
    }

    pub fn instance(
        name: ast::Id,
        comp_loc: GPosIdx,
        bind_loc: GPosIdx,
    ) -> Self {
        Instance {
            name,
            comp_loc,
            bind_loc,
        }
        .into()
    }

    pub fn invoke(name: ast::Id, inst_loc: GPosIdx, bind_loc: GPosIdx) -> Info {
        Invoke {
            name,
            inst_loc,
            bind_loc,
        }
        .into()
    }

    pub fn connect(dst_loc: GPosIdx, src_loc: GPosIdx) -> Self {
        Connect { dst_loc, src_loc }.into()
    }

    pub fn port(
        name: ast::Id,
        bind_loc: GPosIdx,
        width_loc: GPosIdx,
        live_loc: GPosIdx,
    ) -> Self {
        Port {
            name,
            bind_loc,
            width_loc,
            live_loc,
        }
        .into()
    }
}

macro_rules! info_cast {
    ($class:tt, $name:ident) => {
        impl Info {
            pub fn $name(&self) -> Option<&$class> {
                if let Self::$class(v) = self {
                    Some(v)
                } else {
                    None
                }
            }
        }

        impl<'a> From<&'a Info> for &'a $class {
            fn from(info: &'a Info) -> Self {
                if let Info::$class(v) = info {
                    v
                } else {
                    unreachable!(
                        "Incorrect info type for {}.",
                        stringify!($class)
                    );
                }
            }
        }

        impl<'a> From<&'a Info> for Option<&'a $class> {
            fn from(info: &'a Info) -> Self {
                info.$name()
            }
        }
    };
}

info_cast!(Assert, as_assert);
info_cast!(Param, as_param);
info_cast!(Event, as_event);
info_cast!(EventBind, as_event_bind);
info_cast!(Instance, as_instance);
info_cast!(Invoke, as_invoke);
info_cast!(Connect, as_connect);
info_cast!(Port, as_port);

#[derive(Clone, PartialEq, Eq)]
/// Why was an assertion created?
pub enum Reason {
    /// Assertion representing constraint on a parameter
    ParamConstraint {
        /// Location of the binding
        bind_loc: GPosIdx,
        /// Location of the constraint
        constraint_loc: GPosIdx,
    },
    /// Assertion representing constraint on an event
    EventConstraint {
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
        dst_len: ExprIdx,
        src_len: ExprIdx,
    },
    BundleWidthMatch {
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
    EventTrig {
        /// Delay of event of component being triggered
        ev_delay_loc: GPosIdx,
        ev_delay: TimeSub,
        /// Delay of event in containing component
        comp_ev: GPosIdx,
        delay: TimeSub,
        /// Location of the binding
        time_expr_loc: GPosIdx,
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

    pub fn param_cons(bind_loc: GPosIdx, constraint_loc: GPosIdx) -> Self {
        Self::ParamConstraint {
            bind_loc,
            constraint_loc,
        }
    }

    pub fn event_cons(bind_loc: GPosIdx, constraint_loc: GPosIdx) -> Self {
        Self::EventConstraint {
            bind_loc,
            constraint_loc,
        }
    }

    pub fn event_trig(
        ev_delay_loc: GPosIdx,
        ev_delay: TimeSub,
        comp_ev: GPosIdx,
        delay: TimeSub,
        time_expr_loc: GPosIdx,
    ) -> Self {
        Self::EventTrig {
            ev_delay_loc,
            ev_delay,
            comp_ev,
            delay,
            time_expr_loc,
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
            dst_len: dst_width,
            src_len: src_width,
        }
    }

    pub fn bundle_width_match(
        dst_loc: GPosIdx,
        src_loc: GPosIdx,
        dst_width: ExprIdx,
        src_width: ExprIdx,
    ) -> Self {
        Self::BundleWidthMatch {
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
        Self::assert(r)
    }
}

impl Reason {
    /// Convert this reason into a diagnostic message
    pub fn diag(&self, ctx: &Component) -> Diagnostic<usize> {
        match self {
            Reason::Misc { reason, def_loc } => {
                let err = Diagnostic::error().with_message(reason);
                if let Some(loc) = def_loc.into_option() {
                    let label = loc.primary().with_message(reason.clone());
                    err.with_labels(vec![label])
                } else {
                    err
                }
            }
            Reason::ParamConstraint {
                bind_loc,
                constraint_loc,
            } => {
                let con = constraint_loc
                    .primary()
                    .with_message("constraint was violated");
                let inst = bind_loc
                    .secondary()
                    .with_message("instantiation occurs here");
                Diagnostic::error()
                    .with_message("instantiation violates parameter constraint")
                    .with_labels(vec![con, inst])
            }
            Reason::EventConstraint {
                bind_loc,
                constraint_loc,
            } => {
                let con = constraint_loc
                    .primary()
                    .with_message("constraint was violated");
                let inst =
                    bind_loc.secondary().with_message("invocation occurs here");
                Diagnostic::error()
                    .with_message("invocation violates event constraint")
                    .with_labels(vec![con, inst])
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
                dst_len: dst_width,
                src_len: src_width,
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
            Reason::BundleWidthMatch {
                dst_loc,
                src_loc,
                dst_width,
                src_width,
            } => {
                let sw = ctx.display(*src_width);
                let dw = ctx.display(*dst_width);
                let src = src_loc
                    .primary()
                    .with_message(format!("source has width {sw}",));
                let dst = dst_loc
                    .secondary()
                    .with_message(format!("destination has width {dw}",));
                Diagnostic::error()
                    .with_message(format!("required bundle of width `{dw}' but found bundle of width `{sw}'"))
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
                    "available for {} cycles",
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
            Reason::EventTrig {
                ev_delay_loc,
                ev_delay,
                comp_ev,
                delay,
                time_expr_loc,
            } => {
                let bind = time_expr_loc.primary().with_message(
                    "event provided to invoke triggers too often",
                );
                let ev = ev_delay_loc.secondary().with_message(format!(
                    "invocation's event is allowed to trigger every {} cycles",
                    ctx.display_timesub(ev_delay)
                ));
                let comp = comp_ev.secondary().with_message(format!(
                    "this event triggers every {} cycles",
                    ctx.display_timesub(delay)
                ));
                Diagnostic::error()
                    .with_message("event provided to invocation triggers more often that invocation's event's delay allows")
                    .with_labels(vec![bind, ev, comp])
            }
        }
    }
}
