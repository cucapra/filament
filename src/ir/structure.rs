use crate::ast::Op;

use super::{
    Bind, Component, Ctx, Expr, ExprIdx, Foldable, InfoIdx, InvIdx, ParamIdx,
    PortIdx, Subst, TimeIdx, TimeSub,
};
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

impl Foldable<ParamIdx, ExprIdx> for Range {
    type Context = Component;

    fn fold_with<F>(&self, ctx: &mut Self::Context, subst_fn: &mut F) -> Self
    where
        F: FnMut(ParamIdx) -> Option<ExprIdx>,
    {
        Range {
            start: self.start.fold_with(ctx, subst_fn),
            end: self.end.fold_with(ctx, subst_fn),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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

impl fmt::Display for PortOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sig { dir } => write!(f, "sig({})", dir),
            Self::Inv { inv, dir } => write!(f, "{}({})", inv, dir),
            Self::Local => write!(f, "local"),
        }
    }
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
        write!(f, "for<{}: {}> {}", self.idx, self.len, self.range)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// A port tracks its definition and liveness.
/// A port in the IR generalizes both bundles and normal ports.
pub struct Port {
    pub owner: PortOwner,
    pub width: ExprIdx,
    pub live: Liveness,
    pub info: InfoIdx,
}
impl Port {
    /// Check if this is an input port on the signature
    pub fn is_sig_in(&self) -> bool {
        // We check the direction is `out` because the port direction is flipped
        matches!(
            self.owner,
            PortOwner::Sig {
                dir: Direction::Out
            }
        )
    }

    /// Check if this is an output port on the signature
    pub fn is_sig_out(&self) -> bool {
        // We check the direction is `in` because the port direction is flipped
        matches!(self.owner, PortOwner::Sig { dir: Direction::In })
    }
}
impl fmt::Display for Port {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} {}", self.owner, self.live, self.width)
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

    /// Check if this is guaranteed a simple port access, i.e., an access that
    /// produces one port.
    /// The check is syntactic and therefore conservative.
    pub fn is_port(&self, ctx: &mut impl Ctx<Expr>) -> bool {
        let one = ctx.add(Expr::Concrete(1));
        match ctx.get(self.end) {
            Expr::Bin {
                op: Op::Add,
                lhs,
                rhs,
            } => {
                *rhs == one && self.start == *lhs
                    || *lhs == one && self.start == *rhs
            }
            Expr::Concrete(e) => {
                if let Some(s) = self.start.as_concrete(ctx) {
                    *e == s + 1
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Return the bundle type associated with this access
    pub fn bundle_typ(&self, ctx: &mut Component) -> Liveness {
        let live = ctx.get(self.port).live.clone();
        let binding = if self.is_port(ctx) {
            // If this access produces exactly one port, then remap `#idx` to `start`.
            [(live.idx, self.start)]
        } else {
            // Remap `#idx` to `#idx+start
            [(live.idx, live.idx.expr(ctx).add(self.start, ctx))]
        };

        let range = Subst::new(live.range, &Bind::new(&binding)).apply(ctx);
        // Shrink the bundle type based on the access
        let len = self.end.sub(self.start, ctx);
        Liveness {
            idx: live.idx,
            len,
            range,
        }
    }
}
impl fmt::Display for Access {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}..{})", self.port, self.start, self.end)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Construct that defines the parameter
pub enum ParamOwner {
    /// Defined by the signature
    Sig,
    /// Defined by a bundle
    Bundle(PortIdx),
    /// Loop indexing parameter
    Loop,
}

impl ParamOwner {
    pub fn bundle(port: PortIdx) -> Self {
        Self::Bundle(port)
    }
}

impl fmt::Display for ParamOwner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sig => write!(f, "sig"),
            Self::Bundle(p) => write!(f, "bundle({p})"),
            Self::Loop => write!(f, "loop"),
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Parameters with an optional initial value
pub struct Param {
    pub owner: ParamOwner,
    pub info: InfoIdx,
}

impl Param {
    pub fn new(owner: ParamOwner, info: InfoIdx) -> Self {
        Self { owner, info }
    }

    pub fn is_sig_owned(&self) -> bool {
        matches!(self.owner, ParamOwner::Sig)
    }

    pub fn is_local(&self) -> bool {
        matches!(self.owner, ParamOwner::Loop)
    }
}

impl fmt::Display for Param {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.owner)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// The construct that defines an event
pub enum EventOwner {
    /// The event is defined by a signature
    Sig,
    /// The event is defined by an invocation
    Inv { inv: InvIdx },
}

impl EventOwner {
    pub fn is_sig(&self) -> bool {
        matches!(self, Self::Sig)
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
/// Events must have a delay and an optional default value
pub struct Event {
    pub delay: TimeSub,
    pub owner: EventOwner,
    pub info: InfoIdx,
}

impl fmt::Display for Event {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "delay {}", self.delay)
    }
}
