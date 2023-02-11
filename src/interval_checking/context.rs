use super::{FilSolver, ShareConstraints};
use crate::core::{self, Constraint, OrderConstraint, TimeRep, WidthRep};
use crate::errors::FilamentResult;
use crate::utils::GPosIdx;
use crate::visitor;
use itertools::Itertools;
use std::iter;
use std::marker::PhantomData;

type FactMap<T> = Vec<core::Constraint<T>>;

pub struct IntervalCheck<T: TimeRep<Offset = W>, W: WidthRep> {
    /// Solver associated with this context
    pub(super) solver: FilSolver<T>,
    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    pub(super) obligations: FactMap<T>,
    /// Set of assumed facts
    pub(super) facts: FactMap<T>,

    _w: PhantomData<W>,
}

impl<T: TimeRep<Offset = W>, W: WidthRep> From<FilSolver<T>>
    for IntervalCheck<T, W>
{
    fn from(solver: FilSolver<T>) -> Self {
        Self {
            solver,
            obligations: Vec::new(),
            facts: Vec::new(),
            _w: PhantomData,
        }
    }
}

impl<T: TimeRep<Offset = W>, W: WidthRep> IntervalCheck<T, W> {
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

    /// Construct constraints for shared instances.
    /// Add disjointness constraints to the context and returns the set of sharing constraints.
    pub fn drain_sharing(
        &mut self,
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<Vec<ShareConstraints<T>>> {
        let all = ctx
            .instances()
            .map(|inst| self.sharing_constraints(inst, ctx))
            .collect::<FilamentResult<Vec<_>>>()?;
        let mut share = Vec::new();
        for s in all {
            share.extend(s);
        }
        Ok(share)
    }

    /// Get the obligations that need to be proven
    pub fn drain_obligations(&mut self) -> Vec<core::Constraint<T>> {
        std::mem::take(&mut self.obligations)
    }

    /// Generate disjointness constraints for an instance's event bindings.
    /// Get's all the invokes associated with an instance and then ensures that
    /// each binding event occupies a disjoint interval
    fn sharing_constraints(
        &mut self,
        inst: visitor::InstIdx,
        ctx: &visitor::CompBinding<T, W>,
    ) -> FilamentResult<Vec<ShareConstraints<T>>> {
        // Get bindings for all invokes and transpose them so that each inner
        // vector represents the bindings for a single event
        // Reprents invoke -> list (event, delay)
        let invoke_bindings = inst
            .get_all_invokes(ctx)
            .map(|inv| inv.event_active_ranges(ctx))
            .collect_vec();

        // If we don't have multiple invokes, we don't need to generate any
        // constraints
        if invoke_bindings.len() < 2 {
            return Ok(Vec::new());
        }

        // Check that all invokes use the same event binding
        let first_bind = &invoke_bindings[0];
        for binds in &invoke_bindings[1..] {
            for event in 0..first_bind.len() {
                let e1 = first_bind[event].0.event();
                let e2 = binds[event].0.event();
                // If the events are not syntactically equal, add constraint requiring that the events are the same
                if e1 != e2 {
                    let con = Constraint::base(
                        OrderConstraint::eq(
                            T::unit(e1.clone(), 0),
                            T::unit(e2.clone(), 0)
                        )
                    ).add_note(
                        format!(
                        "Invocations of instance use multiple events in invocations: {first} and {event}. Sharing using multiple events is not supported.",
                        first = e1,
                        event = e2),
                        GPosIdx::UNKNOWN,
                    );
                    self.add_obligations(iter::once(con));
                }
            }
        }

        // Iterate over each event
        let events = ctx.prog.events(ctx[inst].sig);
        let mut share_constraints = Vec::new();
        let num_bindings = invoke_bindings.len();
        // for (idx) in ctx.prog.events(sig).iter().enumerate() {
        for event in 0..events.len() {
            // Build up a sharing constraint for each event in the signature.
            // Since all bindings use the same event, we can use the event mentioned in the first binding
            // as the one to use for the sharing constraint
            let bounded_event = first_bind[event].0.event();
            let this = ctx.prog.comp_sig(ctx.sig());
            let mut share =
                ShareConstraints::from(this.get_event(&bounded_event).clone());

            // Iterate over all pairs of bindings
            for i in 0..num_bindings {
                // Add the event binding to the share constraint
                let (start_i, delay) = &invoke_bindings[i][event];
                share.add_bind_info(
                    start_i.clone(),
                    (start_i.clone(), delay.clone()),
                    GPosIdx::UNKNOWN,
                );

                for k in 0..num_bindings {
                    if i == k {
                        continue;
                    }

                    // The bindings must be separated by at least the delay of the first binding
                    // XXX: There probably a more efficient encoding where we ensure that the
                    //      events are max(delay_i, delay_k) cycles apart
                    let (start_k, _) = &invoke_bindings[k][event];
                    let con =
                        core::Constraint::sub(core::OrderConstraint::gte(
                            start_i.clone().sub(start_k.clone()),
                            delay.clone(),
                        ));
                    self.add_obligations(iter::once(con));
                }
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
            share_constraints.push(share);
        }

        Ok(share_constraints)
    }
}
