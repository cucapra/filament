use crate::core::{self, Constraint, OrderConstraint};
use crate::errors::WithPos;
use crate::utils::{FilSolver, ShareConstraint};
use crate::{diagnostics, visitor};
use itertools::Itertools;
use std::iter;

type FactMap = Vec<core::Constraint>;

pub struct IntervalCheck {
    /// Solver associated with this context
    pub(super) solver: FilSolver,
    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    pub(super) obligations: FactMap,
    /// Set of assumed facts
    pub(super) facts: FactMap,
    /// Diagnostics
    pub diag: diagnostics::Diagnostics,
}

impl From<(FilSolver, diagnostics::Diagnostics)> for IntervalCheck {
    fn from((solver, diag): (FilSolver, diagnostics::Diagnostics)) -> Self {
        Self {
            solver,
            obligations: Vec::new(),
            facts: Vec::new(),
            diag,
        }
    }
}

impl IntervalCheck {
    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F)
    where
        F: IntoIterator<Item = core::Constraint>,
    {
        for fact in facts {
            log::trace!("adding obligation {}", fact);
            self.obligations.push(fact);
        }
    }

    /// Add a new known fact
    pub fn add_fact(&mut self, fact: core::Constraint) {
        log::trace!("adding known fact {}", fact);
        self.facts.push(fact);
    }

    /// Construct constraints for shared instances.
    /// Add disjointness constraints to the context and returns the set of sharing constraints.
    pub fn drain_sharing(
        &mut self,
        ctx: &visitor::CompBinding,
    ) -> Vec<ShareConstraint> {
        let all = ctx
            .instances()
            .map(|inst| self.sharing_constraints(inst, ctx))
            .collect_vec();
        let mut share = Vec::new();
        for s in all {
            share.extend(s);
        }
        share
    }

    /// Get the obligations that need to be proven
    pub fn drain_obligations(&mut self) -> Vec<core::Constraint> {
        std::mem::take(&mut self.obligations)
    }

    /// Generate disjointness constraints for an instance's event bindings.
    /// Get's all the invokes associated with an instance and then ensures that
    /// each binding event occupies a disjoint interval
    fn sharing_constraints(
        &mut self,
        inst: visitor::InstIdx,
        ctx: &visitor::CompBinding,
    ) -> Vec<ShareConstraint> {
        // Get bindings for all invokes and transpose them so that each inner
        // vector represents the bindings for a single event
        // Reprents invoke -> list (event, delay)
        let invoke_bindings = inst
            .get_all_invokes(ctx)
            .map(|inv| (inv, inv.event_active_ranges(ctx)))
            .collect_vec();

        // If we don't have multiple invokes, we don't need to generate any
        // constraints
        if invoke_bindings.len() < 2 {
            return Vec::new();
        }

        // Check that all invokes use the same event binding
        let (_, first_bind) = &invoke_bindings[0];
        for (_, binds) in &invoke_bindings[1..] {
            for event in 0..first_bind.len() {
                let e1 = first_bind[event].0.event();
                let e2 = binds[event].0.event();
                // If the events are not syntactically equal, add constraint requiring that the events are the same
                if e1 != e2 {
                    let con = Constraint::base(OrderConstraint::eq(
                        e1.clone().into(),
                        e2.clone().into(),
                    ))
                    .add_note(self.diag.add_info(format!("Invocation uses event {e1}"), e1.copy_span()))
                    .add_note(self.diag.add_info(format!("Invocation uses event {e2}"), e2.copy_span()))
                    .add_note(self.diag.add_message(
                        format!(
                            "Invocations of instance use multiple events in invocations: {e1} and {e2}. Sharing using multiple events is not supported."
                    )));
                    self.add_obligations(iter::once(con));
                }
            }
        }

        // Iterate over each event
        let events = ctx.prog.event_names(ctx[inst].sig);
        let mut share_constraints = Vec::new();
        let num_bindings = invoke_bindings.len();
        for event in 0..events.len() {
            // Build up a sharing constraint for each event in the signature.
            // Since all bindings use the same event, we can use the event mentioned in the first binding
            // as the one to use for the sharing constraint
            let bounded_event = first_bind[event].0.event();
            let this = ctx.prog.comp_sig(ctx.sig());
            let eb = this.get_event(&bounded_event).clone();
            let eb_pos = eb.copy_span();
            let mut share = ShareConstraint::from(eb);

            // Iterate over all pairs of bindings
            for i in 0..num_bindings {
                // Add the event binding to the share constraint
                let (inv_i, binds) = &invoke_bindings[i];
                let (start_i, delay) = &binds[event];
                share.add_bind_info(
                    start_i.clone(),
                    (start_i.clone(), delay.clone()),
                    inv_i.pos(ctx),
                    &mut self.diag,
                );

                // All other bindings must be separated by at least the delay of this binding
                // XXX: There probably a more efficient encoding where we ensure that the
                //      events are max(delay_i, delay_k) cycles apart
                for (k, (inv_k, start_k)) in invoke_bindings
                    .iter()
                    .map(|(inv, b)| (inv, &b[event].0))
                    .enumerate()
                {
                    if i == k {
                        continue;
                    }

                    let con = core::Constraint::sub(
                        core::OrderConstraint::gte(
                            start_i.clone() - start_k.clone(),
                            delay.clone(),
                        ),
                    )
                    .add_note(self.diag.add_info(
                        format!("Delay requires {} cycle between event but reuse may occur after {} cycles", delay.clone(), start_i.clone() - start_k.clone()),
                        eb_pos,
                    ))
                    .add_note(self.diag.add_info(
                        format!("Invocation starts at `{start_k}'"),
                        inv_k.pos(ctx),
                    ))
                    .add_note(self.diag.add_info(
                        format!("Invocation starts at `{start_i}'"),
                        inv_i.pos(ctx),
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

        share_constraints
    }
}
