use super::{Ctx, Expr, ExprIdx, InvIdx, ParamIdx, PortIdx, TimeIdx, TimeSub};
use std::fmt;

#[derive(PartialEq, Eq, Hash, Clone)]
/// An interval of time
pub struct Range {
    pub start: TimeIdx,
    pub end: TimeIdx,
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@[{}, {}]", self.start, self.end)
    }
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
impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Direction::In => write!(f, "in"),
            Direction::Out => write!(f, "out"),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
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

impl fmt::Display for Liveness {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "for<{}> @[{}, {}]",
            self.idx, self.range.start, self.range.end
        )
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// A port tracks its definition and liveness.
/// A port in the IR generalizes both bundles and normal ports.
pub struct Port {
    pub owner: PortOwner,
    pub width: ExprIdx,
    pub live: Liveness,
}
impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.owner {
            PortOwner::Sig { dir } => {
                write!(f, "{} {} {}", dir, self.live, self.width)
            }
            PortOwner::Inv { inv, dir } => {
                write!(f, "{} {} {} {}", dir, inv, self.live, self.width)
            }
            PortOwner::Local => {
                write!(f, "{} {}", self.live, self.width)
            }
        }
    }
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
impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}..{})", self.port, self.start, self.end)
    }
}

#[derive(Default, PartialEq, Eq, Hash, Clone)]
/// Parameters with an optional initial value
pub struct Param;

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "()")
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Events must have a delay and an optional default value
pub struct Event {
    pub delay: TimeSub,
    pub default: Option<TimeIdx>,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(default) = self.default {
            write!(f, "{} ", default)?;
        }
        write!(f, "delay {}", self.delay)
    }
}
