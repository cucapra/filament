use super::{EventIdx, ExprIdx, RangeIdx, TimeIdx};

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

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
/// A port tracks its definition and liveness.
/// A port in the IR generalizes both bundles and normal ports.
pub struct Port {
    pub len: ExprIdx,
    pub width: ExprIdx,
    pub liveness: RangeIdx,
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
