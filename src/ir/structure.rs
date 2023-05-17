use super::{EventIdx, ExprIdx, InvIdx, ParamIdx, RangeIdx, TimeIdx};

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// A temporal event. Represents an offset from the start of the event.
pub struct Time {
    pub event: EventIdx,
    pub offset: ExprIdx,
}

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
    pub range: RangeIdx,
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

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// Events must have a delay and an optional default value
pub struct Event {
    pub delay: ExprIdx,
    pub default: Option<ExprIdx>,
}
