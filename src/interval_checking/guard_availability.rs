use itertools::Itertools;

use super::{Context, THIS};
use crate::core::{FsmIdxs, TimeRep};
use crate::errors::{self, Error, FilamentResult, WithPos};
use crate::event_checker::ast;

type Intervals = Vec<(ast::Interval, Option<errors::Span>)>;

/// Computes the availablity of guards.
fn guard_availability(
    guard: &ast::Guard,
    ctx: &Context,
    acc: &mut Intervals,
) -> FilamentResult<()> {
    match guard {
        ast::Guard::Or(g1, g2, _) => {
            guard_availability(g1, ctx, acc)?;
            guard_availability(g2, ctx, acc)?;
            Ok(())
        }
        ast::Guard::Port(p) => {
            let interval = match &p.typ {
                ast::PortType::ThisPort(name) => {
                    ctx.get_invoke(&THIS.into())?.port_guarantees(name)?
                }
                ast::PortType::InvPort { invoke: comp, name } => {
                    ctx.get_invoke(comp)?.port_guarantees(name)?
                }
                ast::PortType::Constant(_) => {
                    return Err(Error::malformed(
                        "Guards cannot contain constants",
                    )
                    .add_note(
                        "Guards cannot contain constants",
                        p.copy_span(),
                    ))
                }
            };

            if interval.exact.is_none() {
                return Err(Error::malformed(
                    "Ports used in guards must have exact guarantees",
                )
                .add_note(
                    format!("Port provides guarantee {}.", interval),
                    p.copy_span(),
                ));
            }
            acc.push((interval, p.copy_span()));
            Ok(())
        }
    }
}

/// Merges exactly available signals into one interval.
/// All intervals need to be based on the same time variable and only be increments.
fn merge_availability(intervals: Intervals) -> FilamentResult<ast::Interval> {
    // Check that all within intervals are the same.
    let mut interval_iter = intervals.iter();
    if let Some((int_av0, av0_pos)) = interval_iter.next() {
        if let Some((diff, diff_pos)) =
            interval_iter.find(|(int, _)| int.within != int_av0.within)
        {
            return Err(Error::malformed(
                "Guard ports are available for different time intervals",
            )
            .add_note(
                format!("Port is available for {}", diff),
                diff_pos.clone(),
            )
            .add_note(
                format!("Port is available for {}", int_av0),
                av0_pos.clone(),
            ));
        }
    } else {
        unreachable!("Should provide at least one interval")
    }
    let within = intervals[0].0.within.clone();

    let exacts: Vec<_> = intervals
        .iter()
        .map(|(iv, pos)| {
            iv.as_exact_offset().ok_or_else(|| {
                Error::malformed(format!(
                    "Cannot convert interval into offset: {iv}",
                ))
                .add_note(
                    format!("Cannot convert @exact specification into an offset: {iv}"),
                    pos.clone(),
                )
            }).map(|v| (v, pos))
        })
        .collect::<FilamentResult<_>>()?;

    let mut ex_iter = exacts.into_iter().sorted_by(|ev1, ev2| ev1.1.cmp(ev2.1));
    let first = ex_iter.next().unwrap();
    let ((event, start, end), _) = ex_iter.try_fold(
        first,
        |((event, start, end), prev_pos), ((ev, s, e), cur_pos)| {
            if event != ev {
                Err(Error::malformed(
                    "Intervals use different events: {event} and {ev}",
                )
                .add_note(format!("Event is {event}"), prev_pos.clone())
                .add_note(format!("Event is {ev}"), cur_pos.clone()))
            } else if s != end {
                Err(Error::malformed(
                    "Guard @exact specification should not overlap",
                )
                .add_note(
                    format!("Interval start is is {ev}+{s}"),
                    cur_pos.clone(),
                )
                .add_note(
                    format!("Interval end is {ev}+{end}"),
                    prev_pos.clone(),
                ))
            } else {
                Ok(((event, start, e), cur_pos))
            }
        },
    )?;

    Ok(ast::Interval::from(within).with_exact(ast::Range::new(
        FsmIdxs::unit(event.clone(), start),
        FsmIdxs::unit(event.clone(), end),
    )))
}

pub fn total_interval(
    guard: &ast::Guard,
    ctx: &Context,
) -> FilamentResult<ast::Interval> {
    let mut intervals = vec![];
    guard_availability(guard, ctx, &mut intervals)?;
    merge_availability(intervals)
}
