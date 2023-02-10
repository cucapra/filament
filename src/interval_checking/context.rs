use super::{FilSolver, ShareConstraints};
use crate::core::{self, Constraint, TimeRep, WidthRep, WithTime};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor;
use std::collections::HashMap;

type FactMap<T> = Vec<core::Constraint<T>>;
type BindsWithLoc<T> = (GPosIdx, Vec<T>);

pub struct IntervalCheck<T: TimeRep> {
    /// Solver associated with this context
    pub(super) solver: FilSolver<T>,
    /// Mapping from instance to event bindings
    pub(super) event_binds: HashMap<core::Id, Vec<BindsWithLoc<T>>>,
    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    pub(super) obligations: FactMap<T>,
    /// Set of assumed facts
    pub(super) facts: FactMap<T>,
}

impl<T: TimeRep> From<FilSolver<T>> for IntervalCheck<T> {
    fn from(solver: FilSolver<T>) -> Self {
        Self {
            solver,
            event_binds: HashMap::new(),
            obligations: Vec::new(),
            facts: Vec::new(),
        }
    }
}

impl<T: TimeRep> IntervalCheck<T> {
    /// Track event bindings for each instance.
    /// This is used for the disjointness check.
    pub fn add_event_binds(
        &mut self,
        instance: core::Id,
        binds: &core::Binding<T>,
        pos: GPosIdx,
    ) {
        self.event_binds
            .entry(instance)
            .or_default()
            .push((pos, binds.iter().map(|(_, t)| t.clone()).collect()));
    }

    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F)
    where
        F: Iterator<Item = core::Constraint<T>>,
    {
        for fact in facts {
            log::trace!("adding obligation {}", fact);
            self.obligations.push(fact);
        }
    }

    /// Add a new known fact
    pub fn add_fact(&mut self, fact: core::Constraint<T>) {
        log::trace!("adding known fact {}", fact);
        self.facts.push(fact);
    }

    /// Construct disjointness constraints for the current context
    pub fn drain_sharing<W: WidthRep>(
        &mut self,
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<(Vec<Constraint<T>>, Vec<ShareConstraints<T>>)> {
        let evs = std::mem::take(&mut self.event_binds);
        let all = evs
            .into_iter()
            .map(|(inst, binds)| Self::sharing_constraints(inst, &binds, ctx))
            .collect::<FilamentResult<Vec<_>>>()?;
        let mut cons = Vec::new();
        let mut share = Vec::new();
        for (c, s) in all {
            cons.extend(c);
            share.extend(s);
        }
        Ok((cons, share))
    }

    /// Get the obligations that need to be proven
    pub fn drain_obligations(&mut self) -> Vec<core::Constraint<T>> {
        std::mem::take(&mut self.obligations)
    }

    fn ensure_same_events<W: WidthRep>(
        instance: &core::Id,
        args: &[BindsWithLoc<T>],
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<()> {
        // Get the delay associated with each event.
        let sig = ctx.get_instance(instance).sig;
        // Ensure that all bindings of an event variable use the same events
        for (idx, eb) in ctx.prog.events(sig).iter().enumerate() {
            let mut iter = args.iter().map(|(pos, binds)| (pos, &binds[idx]));
            let abs = &eb.event;

            if let Some((fpos, first)) = iter.next() {
                for (epos, event) in iter {
                    if event.event() != first.event() {
                        return Err(Error::malformed(format!(
                                "Invocations of instance `{instance}' use multiple events for event `{abs}' binding: {first} and {event}. Sharing using multiple events is not supported.",
                            ))
                            .add_note(format!("Location provides binding {instance}.{abs}={first}"), *fpos)
                            .add_note(format!("Location provides binding {instance}.{abs}={event}"), *epos));
                    }
                }
            }
        }
        Ok(())
    }

    /// Constraint generated for disjointness
    fn sharing_contraints(
        instance: &core::Id,
        abs: &core::Id,
        (i_event, spi): (&T, GPosIdx),
        (k_event, spk): (&T, GPosIdx),
        i_delay: &T::SubRep,
        id_pos: GPosIdx,
    ) -> core::Constraint<T> {
        core::Constraint::sub(core::OrderConstraint::gte(
            i_event.clone().sub(k_event.clone()),
            i_delay.clone(),
        ))
        .add_note(
            format!("Conflicting invoke. Invoke provides binding {instance}.{abs}={k_event}"),
            spk,
        )
        .add_note(
            format!("Invoke provides binding {instance}.{abs}={i_event}"),
            spi,
        )
        .add_note(
            format!("Delay for {abs} specifies that invokes must be {i_delay} cycles apart"),
            id_pos,
        )
    }

    /// Generate disjointness constraints for an instance's event bindings.
    fn sharing_constraints<W: WidthRep>(
        instance: core::Id,
        args: &[BindsWithLoc<T>],
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<(Vec<Constraint<T>>, Vec<ShareConstraints<T>>)> {
        // Ensure that bindings for events variables use the same variables.
        Self::ensure_same_events(&instance, args, ctx)?;

        // Get the delay associated with each event.
        let sig = ctx.get_instance(&instance).sig;

        // Iterate over each event
        let mut constraints = Vec::new();
        let mut share_constraints = Vec::new();
        for (idx, eb) in ctx.prog.events(sig).iter().enumerate() {
            // Track minimum and maximum end times for each binding
            let mut share = ShareConstraints::default();

            let delay = &eb.delay;
            // For each binding
            for (i, (spi, bi)) in args.iter().enumerate() {
                // Delay implied by the i'th binding
                let i_delay =
                    delay.resolve_event(&ctx.prog.event_binding(sig, bi));
                // The i'th use conflicts with all other uses
                for (k, (spk, bk)) in args.iter().enumerate() {
                    if i == k {
                        continue;
                    }

                    constraints.push(Self::sharing_contraints(
                        &instance,
                        &eb.event,
                        (&bi[idx], *spi),
                        (&bk[idx], *spk),
                        &i_delay,
                        eb.copy_span(),
                    ))
                }
                share.add_bind_info(
                    bi[idx].clone(),
                    (bi[idx].clone(), i_delay),
                    *spi,
                );
            }

            // # Constraints generated from sharing instances
            // If a instance is shared at events Gi, then for all events T defined by
            // the instance, where dT defines the delay for T, and dG is the delay for
            // G, we have:
            // dG >= max(Gi+dT.resolve(Gi)) - min(Gi)
            //
            // In other words, the delay of the events trigger the shared instance
            // should be greater that the range occupied by the invocations of the
            // instance.
            let this_sig = ctx.sig();
            // Get delays for events used in bindings. These are guaranteed to be the same across all bindings
            // due to the call to `ensure_same_events`.
            let bind = &args[0].1[idx];
            let ev = &bind.event();
            let eb = ctx.prog.get_event(this_sig, ev);
            share.add_delays(std::iter::once(eb.clone()));
            share_constraints.push(share);
        }

        Ok((constraints, share_constraints))
    }
}
