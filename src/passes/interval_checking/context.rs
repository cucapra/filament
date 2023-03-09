use crate::core::{self, Constraint, OrderConstraint};
use crate::utils::{self, FilSolver, ShareConstraint};
use crate::{binding, diagnostics};
use itertools::Itertools;
use std::iter;

type FactMap = Vec<core::Constraint>;

pub struct IntervalCheck {
    /// Solver associated with this context
    pub(super) solver: FilSolver,
    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    pub(super) obligations: Vec<utils::Obligation>,
    /// Set of assumed facts
    pub(super) facts: FactMap,
    /// Diagnostics
    pub diag: diagnostics::Diagnostics,
    /// Variables used in the current set of constraints
    pub(super) vars: Vec<core::Id>,
}

impl IntervalCheck {
    /// Construct a new context
    pub fn new(solver: FilSolver, diag: diagnostics::Diagnostics) -> Self {
        Self {
            vars: Vec::new(),
            obligations: Vec::new(),
            facts: Vec::new(),
            diag,
            solver,
        }
    }
}

impl IntervalCheck {
    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F)
    where
        F: IntoIterator<Item = utils::Obligation>,
    {
        for fact in facts {
            // log::trace!("adding obligation {}", fact);
            self.obligations.push(fact);
        }
    }

    /// Add a new known fact
    pub fn add_facts<F>(&mut self, facts: F)
    where
        F: IntoIterator<Item = core::Constraint>,
    {
        for fact in facts {
            log::trace!("adding known fact {}", fact);
            self.facts.push(fact);
        }
    }

    /// Add a new variable to the context
    pub fn add_var(&mut self, var: core::Id) {
        self.vars.push(var);
    }

    /// Iterator over the variables
    pub fn vars(&self) -> impl Iterator<Item = core::Id> + '_ {
        self.vars.iter().cloned()
    }

    /// Construct constraints for shared instances.
    /// Add disjointness constraints to the context and returns the set of sharing constraints.
    pub fn drain_sharing(
        &mut self,
        ctx: &binding::CompBinding,
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
    pub fn drain_obligations(&mut self) -> Vec<utils::Obligation> {
        std::mem::take(&mut self.obligations)
    }

    /// Generate disjointness constraints for an instance's event bindings.
    /// Get's all the invokes associated with an instance and then ensures that
    /// each binding event occupies a disjoint interval
    fn sharing_constraints(
        &mut self,
        inst: binding::InstIdx,
        ctx: &binding::CompBinding,
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
        let (inv1, first_bind) = &invoke_bindings[0];
        for (inv2, binds) in &invoke_bindings[1..] {
            for event in 0..first_bind.len() {
                if inv1.is_inferred(ctx, event) && inv2.is_inferred(ctx, event)
                {
                    continue;
                }

                let ev1 = &first_bind[event].0;
                let e1 = ev1.event();
                let ev2 = &binds[event].0;
                let e2 = ev2.event();
                // If the events are not syntactically equal, add constraint requiring that the events are the same
                if e1 != e2 {
                    let con = Constraint::base(OrderConstraint::eq(
                        e1.into(),
                        e2.into(),
                    ))
                    .obligation(
                        format!(
                            "invocations of instance use multiple events in invocations: {e1} and {e2}",
                        )
                    )
                    .add_note(self.diag.add_info(format!("invocation uses event {e1}"), ev1.pos()))
                    .add_note(self.diag.add_info(format!("invocation uses event {e2}"), ev2.pos()));
                    self.add_obligations(iter::once(con));
                }
            }
        }

        // Iterate over each event
        let event_binds = &ctx.prog[ctx[inst].sig].events;
        let mut share_constraints = Vec::new();
        let num_bindings = invoke_bindings.len();
        for event in 0..event_binds.len() {
            // Build up a sharing constraint for each event in the signature.
            // Since all bindings use the same event, we can use the event mentioned in the first binding
            // as the one to use for the sharing constraint
            let bounded_event = first_bind[event].0.event();
            let this = ctx.this();
            let eb = this.get_event(&bounded_event).clone();
            let mut share = ShareConstraint::from(eb);

            // Iterate over all pairs of bindings
            for i in 0..num_bindings {
                // Add the event binding to the share constraint
                let (inv_i, binds) = &invoke_bindings[i];
                let (start_i, delay) = &binds[event];

                // If this is not an inferred binding, add it to the share constraint
                if !inv_i.is_inferred(ctx, event) {
                    share.add_bind_info(
                        start_i.clone(),
                        (start_i.inner().clone(), delay.inner().clone()),
                        &mut self.diag,
                    );
                }

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
                    // If both the events are inferred, don't generate constraints
                    if inv_i.is_inferred(ctx, event)
                        && inv_k.is_inferred(ctx, event)
                    {
                        continue;
                    }

                    let diff =
                        start_i.inner().clone() - start_k.inner().clone();

                    let bind_pos = event_binds[event].delay.pos();
                    let con = core::Constraint::sub(
                        core::OrderConstraint::gte(
                            diff.clone(),
                            delay.inner().clone(),
                        ),
                    )
                    .obligation("instance must be shared with sufficient delay")
                    .add_note(self.diag.add_info(
                        format!("delay requires {} cycle between event but reuse may occur after {} cycles", delay.clone(), diff),
                        bind_pos
                    ))
                    .add_note(self.diag.add_info(
                        format!("invocation starts at `{start_k}'"),
                        inv_k.pos(ctx),
                    ))
                    .add_note(self.diag.add_info(
                        format!("invocation starts at `{start_i}'"),
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
            if !share.is_empty() {
                share_constraints.push(share);
            }
        }

        share_constraints
    }
}
