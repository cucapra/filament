use super::Id;

/// Possible operations over time variables.
pub enum TimeOp {
    Add,
    Sub,
}

/// Represents a time variable which can either be:
///   1. An abstract variable like `G`.
///   2. A concrete time such as 1.
///   3. A binary operation of two other interval times.
pub enum IntervalTime {
    Abstract(Id),
    Concrete(u64),
    BinOp {
        op: TimeOp,
        left: Box<IntervalTime>,
        right: Box<IntervalTime>,
    },
}

/// Type of the interval which can either be:
///   1. Exact, implying set equality
///   2. Within, implying set containment.
pub enum IntervalType {
    Exact,
    Within,
}

/// An interval consists of a type tag, a start time, and a end time.
pub struct Interval {
    pub tag: IntervalType,
    pub start: IntervalTime,
    pub end: IntervalTime,
}
