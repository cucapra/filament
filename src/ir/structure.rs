use super::{EventIdx, ExprIdx, InvIdx, ParamIdx, TimeIdx, TimeSub};

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
    /// The port is defined locally
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

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// Parameters with an optional initial value
pub struct Param {
    pub default: Option<ExprIdx>,
}
impl Param {
    pub fn new() -> Self {
        Self { default: None }
    }
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
