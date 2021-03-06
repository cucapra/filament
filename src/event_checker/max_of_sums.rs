use crate::{core, frontend};

/// Reduces an IntervalTime expression into a max of sums representation.
/// The returned vector represents all the non-max IntervalTime expressions of
/// which the max is being computed.
pub fn max_of_sums(event: frontend::IntervalTime, acc: &mut core::FsmIdxs) {
    use frontend::IntervalTime::*;
    match event {
        Abstract(name) => acc.insert(name, 0),
        Concrete(_) => {
            panic!("Concrete interval time reached while computing max of sums")
        }
        Max { left, right } => {
            max_of_sums(*left, acc);
            max_of_sums(*right, acc);
        }
        Add { left, right } => {
            match (*left, *right) {
                (Concrete(n), e) | (e, Concrete(n)) => {
                    match e {
                        Abstract(name) => acc.insert(name, n),
                        Max { left, right } => {
                            // XXX(rachit): This can probably use the add method on FsmIdxs
                            let left_sum = frontend::IntervalTime::binop_add(*left, n.into());
                            max_of_sums(left_sum, acc);
                            let right_sum = frontend::IntervalTime::binop_add(*right, n.into());
                            max_of_sums(right_sum, acc);
                        }
                        Add {  .. } => panic!("Add expressions are nested, should've been reduced"),
                        Concrete(_) => panic!("Event add expression is sum of two values, should've been reduced already")
                    }
                }
                _ => panic!("Event add expression does not have a nat"),
            }
        }
    }
}

impl From<frontend::IntervalTime> for core::FsmIdxs {
    fn from(time: frontend::IntervalTime) -> Self {
        let mut out = core::FsmIdxs::default();
        max_of_sums(time, &mut out);
        out
    }
}
