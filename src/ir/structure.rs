use super::{
    Ctx, EventIdx, Expr, ExprIdx, InvIdx, ParamIdx, PortIdx, TimeIdx, TimeSub,
};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// An interval of time
pub struct Range {
    pub start: TimeIdx,
    pub end: TimeIdx,
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// The context in which a port was defined.
pub enum PortOwner {
    /// The port is defined in the signature
    Sig { dir: Direction },
    /// The port is defined by an invocation
    Inv { inv: InvIdx, dir: Direction },
    /// The port is defined locally.
    /// It does not have a direction because both reading and writing to it is allowed.
    Local,
}
impl PortOwner {
    /// Input on the signature
    pub const fn sig_in() -> Self {
        Self::Sig { dir: Direction::In }
    }

    /// Output on the signature
    pub const fn sig_out() -> Self {
        Self::Sig {
            dir: Direction::Out,
        }
    }

    /// An input port created by an invocation
    pub const fn inv_in(inv: InvIdx) -> Self {
        Self::Inv {
            inv,
            dir: Direction::In,
        }
    }

    /// An output port created by an invocation
    pub const fn inv_out(inv: InvIdx) -> Self {
        Self::Inv {
            inv,
            dir: Direction::Out,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Direction {
    /// Input port
    In,
    /// Output port
    Out,
}

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// Duration when the port caries a meaningful value.
/// Equivalent to the bundle type:
/// ```
/// p[N]: for<#i> @[G, G+i+10]
/// ```
pub struct Liveness {
    pub idx: ParamIdx,
    pub len: ExprIdx,
    pub range: Range,
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// A port tracks its definition and liveness.
/// A port in the IR generalizes both bundles and normal ports.
pub struct Port {
    pub owner: PortOwner,
    pub width: ExprIdx,
    pub live: Liveness,
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Represents a port access in bundle syntax since the IR desugars all ports to
/// bundles.
pub struct Access {
    pub port: PortIdx,
    /// The start of the access range (inclusive)
    pub start: ExprIdx,
    /// The end of the access range (exclusive)
    pub end: ExprIdx,
}
impl Access {
    /// Construct an access on a simple port (i.e. not a bundle)
    /// The access only indexes into the first element of the port.
    pub fn port(port: PortIdx, ctx: &mut impl Ctx<Expr>) -> Self {
        let zero = ctx.add(Expr::Concrete(0));
        let one = ctx.add(Expr::Concrete(1));
        Self {
            port,
            start: zero,
            end: one,
        }
    }

    /// Check if this is a simple port access
    pub fn is_port(&self, ctx: &impl Ctx<Expr>) -> bool {
        if let (Some(s), Some(e)) =
            (self.start.as_concrete(ctx), self.end.as_concrete(ctx))
        {
            s == 0 && e == 1
        } else {
            false
        }
    }
}

#[derive(Default, PartialEq, Eq, Hash, Clone, Debug)]
/// Parameters with an optional initial value
pub struct Param {
    // XXX(rachit): Should we have default values at this level or should they be
    // compiled away by the time we generate the IR.
    pub default: Option<ExprIdx>,
}
impl Param {
    /// Construct a parameter with a default value.
    pub fn with_default(default: ExprIdx) -> Self {
        Self {
            default: Some(default),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Events must have a delay and an optional default value
pub struct Event {
    pub delay: TimeSub,
    pub default: Option<TimeIdx>,
}
