use itertools::Itertools;

use super::{Context, THIS};
use crate::{core, errors::FilamentResult};

type Intervals = Vec<core::Interval<super::TimeRep>>;

/// Computes the availablity of guards.
fn guard_availability(
    guard: &core::Guard,
    ctx: &Context,
    acc: &mut Intervals,
) -> FilamentResult<()> {
    match guard {
        core::Guard::Or(g1, g2) => {
            guard_availability(g1, ctx, acc)?;
            guard_availability(g2, ctx, acc)?;
            Ok(())
        }
        core::Guard::Port(p) => {
            let interval = match p {
                core::Port::ThisPort(name) => {
                    ctx.get_invoke(&THIS.into())?.port_guarantees(name)?
                }
                core::Port::CompPort { comp, name } => {
                    ctx.get_invoke(comp)?.port_guarantees(name)?
                }
                core::Port::Constant(_) => {
                    panic!("constants cannot appear in guard conditions")
                }
            };
            if interval.typ != core::ITag::Exact {
                panic!("Only exact guarantees are allowed in port")
            }
            acc.push(interval);
            Ok(())
        }
    }
}

/// Merges exactly available signals into one interval.
/// All intervals need to be based on the same time variable and only be increments.
fn merge_availability(intervals: Intervals) -> core::Interval<super::TimeRep> {
    assert!(
        !intervals.is_empty(),
        "Cannot compute availablity with empty intervals"
    );

    // The event over which we're computing availablity
    let mut event = None;
    // Range of FSM states
    let mut ranges = Vec::with_capacity(intervals.len());

    // Check if all intervals contain exactly one FSM and use the same events.
    for interval @ core::Interval { start, end, .. } in &intervals {
        let s_events = start.events().collect_vec();
        let e_events = end.events().collect_vec();
        assert!(
            s_events.len() == 1,
            "Interval should only use one event: {:?}",
            start
        );
        assert!(
            e_events.len() == 1,
            "Interval should only use one event: {:?}",
            end
        );
        assert!(
            s_events == e_events,
            "Interval should use the same events: {:?}",
            interval
        );
        if let Some(ev) = &event {
            assert!(s_events[0] == ev, "Intervals use different events");
        } else {
            event = Some(s_events[0].clone());
        }
        ranges.push((start.fsms[0].state, end.fsms[0].state));
    }

    // Sort ranges by start time
    let (start, end) = ranges
        .into_iter()
        .sorted_by(|ev1, ev2| ev1.0.cmp(&ev2.0))
        .reduce(|(start, end), (s, e)| {
            assert!(s == end, "Ranges are not continuous");
            (start, e)
        })
        .unwrap();

    core::Interval::exact(
        core::FsmIdxs::unit(event.clone().unwrap(), start),
        core::FsmIdxs::unit(event.unwrap(), end),
    )
}

pub fn total_interval(
    guard: &core::Guard,
    ctx: &Context,
) -> FilamentResult<core::Interval<super::TimeRep>> {
    let mut intervals = vec![];
    guard_availability(guard, ctx, &mut intervals)?;
    Ok(merge_availability(intervals))
}
