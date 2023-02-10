use super::{ConcreteInvoke, Context, FilSolver, THIS};
use crate::core::{self, OrderConstraint, TimeRep, WidthRep, WithTime};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor;
use itertools::Itertools;
use std::collections::HashMap;
use std::iter;

impl<T: TimeRep, W: WidthRep> Context<'_, T, W> {
    // For connect statements of the form:
    // dst = src
    // The generated proof obligation requires that req(dst) \subsetof guarantees(src)
    fn check_connect(
        &mut self,
        dst: &core::Port,
        src: &core::Port,
    ) -> FilamentResult<()> {
        log::trace!("Checking connect: {} = {}", dst, src);
        // Remove dst from remaining ports
        self.remove_remaning_assign(dst)?;

        let requirement = self.port_requirements(dst)?;
        let guarantee = self.port_guarantees(src)?;
        let src_pos = src.copy_span();

        // If we have: dst = src. We need:
        // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
        // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
        if let Some(guarantee) = &guarantee {
            let within_fact =
                OrderConstraint::subset(requirement.clone(), guarantee.clone())
                    .map(|e| {
                        core::Constraint::base(e)
                            .add_note(
                                format!(
                                    "Source is available for {}",
                                    guarantee
                                ),
                                src_pos,
                            )
                            .add_note(
                                format!(
                                    "Destination's requirement {}",
                                    requirement
                                ),
                                dst.copy_span(),
                            )
                    });
            self.add_obligations(within_fact);
        }

        Ok(())
    }

    fn check_invoke_binds(
        &mut self,
        invoke: &core::Invoke<T>,
    ) -> FilamentResult<()> {
        let binding = self
            .get_instance(&invoke.instance)
            .event_binding(&invoke.abstract_vars);

        // Track event bindings
        self.add_event_binds(
            invoke.instance.clone(),
            &binding,
            invoke.copy_span(),
        );

        let inv = self.get_invoke(&THIS.into());
        let mut constraints = vec![];

        // For each event provided in the bining, ensure that the corresponding interface
        // does not pulse more often than the interface allows.
        for (abs, evs) in binding.iter() {
            if let Some(inst_event) =
                self.get_instance(&invoke.instance).get_event(abs)
            {
                // Each event in the binding must pulse less often than the interface of the abstract
                // variable.
                let event = &evs.event();
                // Get interface for this event
                let event_interface = inv.get_event(event);
                let int_len = inst_event.delay.resolve_event(&binding);
                let ev_int_len = &event_interface.delay;

                // Generate constraint
                let cons = core::Constraint::sub(core::OrderConstraint::gte(
                    ev_int_len.clone(),
                    int_len.clone(),
                )).add_note(
                    "Event provided to invoke pulses more often than event allows",
                    invoke.copy_span(),
                )
                .add_note(
                    format!(
                        "Provided event may trigger every {} cycles",
                        ev_int_len,
                    ),
                    event_interface.copy_span(),
                )
                .add_note(
                    format!(
                        "Delay requires event to trigger once in {} cycles",
                        int_len,
                    ),
                    inst_event.copy_span(),
                );
                constraints.push(cons);
            }
        }
        self.add_obligations(constraints.into_iter());

        Ok(())
    }

    /// Check invocation and add new [super::Fact] representing the proof obligations for checking this
    /// invocation.
    fn check_invoke(&mut self, invoke: &core::Invoke<T>) -> FilamentResult<()> {
        // Check the bindings for abstract variables do not violate @interface
        // requirements
        self.check_invoke_binds(invoke)?;
        let sig = self.get_instance(&invoke.instance).resolve()?;
        let binding = sig.event_binding(&invoke.abstract_vars);

        // Handle `where` clause constraints and well formedness constraints on intervals.
        sig.well_formed()?.for_each(|con| {
            self.add_obligations(iter::once(con.resolve_event(&binding)).map(
                |e| {
                    e.add_note(
                        "Invoke's intervals must be well-formed",
                        invoke.copy_span(),
                    )
                },
            ))
        });

        sig.constraints.iter().for_each(|con| {
            self.add_obligations(iter::once(con.resolve_event(&binding)).map(
                |e| {
                    e.add_note(
                    "Component's where clause constraints must be satisfied",
                    invoke.copy_span(),
                )
                },
            ))
        });

        // Add this invocation to the context
        self.add_invocation(
            invoke.name.clone(),
            ConcreteInvoke::concrete(binding, sig),
        );

        Ok(())
    }

    fn check_invoke_ports(
        &mut self,
        invoke: &core::Invoke<T>,
    ) -> FilamentResult<()> {
        let sig = self.get_instance(&invoke.instance).resolve()?;
        // If this is a high-level invoke, check all port requirements
        if let Some(actuals) = &invoke.ports {
            // Check connections implied by the invocation
            for (actual, formal) in actuals.iter().zip(sig.inputs()) {
                let dst =
                    core::Port::comp(invoke.name.clone(), formal.name.clone())
                        .set_span(formal.copy_span());
                self.check_connect(&dst, actual)?;
            }
        } else {
            self.add_remaning_assigns(invoke.name.clone(), &invoke.instance)?;
        }

        Ok(())
    }

    fn check_commands(
        &mut self,
        cmds: &[core::Command<T, W>],
    ) -> FilamentResult<()> {
        // Walk over the commands and add bindings for all invocations
        for cmd in cmds {
            match cmd {
                core::Command::Invoke(invoke) => self.check_invoke(invoke)?,
                core::Command::Instance(core::Instance {
                    name,
                    component,
                    bindings,
                    ..
                }) => self.add_instance(name.clone(), component, bindings),
                core::Command::Fsm(_) | core::Command::Connect(_) => (),
            };
        }

        // Check port availability for all connections
        for cmd in cmds {
            match cmd {
                core::Command::Invoke(invoke) => {
                    self.check_invoke_ports(invoke)?
                }
                core::Command::Connect(core::Connect {
                    dst,
                    src,
                    guard,
                    ..
                }) => {
                    if guard.is_none() {
                        self.check_connect(dst, src)?
                    }
                }
                core::Command::Instance(_) | core::Command::Fsm(_) => (),
            };
        }
        Ok(())
    }

    fn check_component(
        solver: &mut FilSolver<T>,
        comp: &core::Component<T, W>,
        sigs: &visitor::Bindings<T, W>,
    ) -> FilamentResult<()> {
        let mut ctx = Context::from(sigs);

        // Ensure that the signature is well-formed
        ctx.add_obligations(comp.sig.well_formed()?);

        // Add instance for this component. Whenever a bare port is used, it refers
        // to the port on this instance.
        let rev_sig = comp.sig.reversed();
        let this_instance = ConcreteInvoke::this_instance(&rev_sig);
        ctx.add_invocation(THIS.into(), this_instance);

        // User-level components are not allowed to have ordering constraints. See https://github.com/cucapra/filament/issues/27.
        for constraint in &rev_sig.constraints {
            if constraint.is_ordering() {
                return Err(Error::malformed(
                    "User-level components cannot have ordering constraints",
                )
                .add_note(
                    format!("Component defines the constraint {constraint}"),
                    GPosIdx::UNKNOWN,
                ));
            } else {
                ctx.add_fact(constraint.clone())
            }
        }

        // Check all the commands
        let t = std::time::Instant::now();
        ctx.check_commands(&comp.body)?;
        log::info!(
            "interval-check.{}.cmds: {}ms",
            comp.sig.name,
            t.elapsed().as_millis()
        );
        // Add obligations from disjointness constraints
        let (disj, share) = ctx.drain_sharing()?;

        // Prove all the required obligations
        let obs = ctx.drain_obligations();
        let t = std::time::Instant::now();
        solver.prove(
            comp.sig.events().chain(comp.sig.params.iter().cloned()),
            // XXX(rachit): Unnecessary clone
            ctx.facts.clone(),
            obs.into_iter().chain(disj.into_iter()),
            share,
        )?;
        log::info!(
            "interval-check.{}.prove: {}ms",
            comp.sig.name,
            t.elapsed().as_millis()
        );

        // There should be no remaining assignments after checking a component
        if let Some((comp, ports)) = ctx.get_remaining_assigns().next() {
            return Err(Error::malformed(format!(
                "Assignment for invoke missing: {}.{}",
                comp,
                ports.iter().next().unwrap()
            )));
        }

        Ok(())
    }
}

/// Check a [core::Namespace] to prove that the interval requirements of all the ports can be
/// satisfied.
/// Internally generates [super::Fact] which represent proof obligations that need to be proven for
/// the interval requirements to be proven.
pub fn check<T: TimeRep, W: WidthRep>(
    mut ns: core::Namespace<T, W>,
) -> FilamentResult<core::Namespace<T, W>> {
    let comps = ns.components.drain(..).collect_vec();
    let sigs: HashMap<_, _> = ns.signatures().collect();
    let mut solver = FilSolver::new()?;

    // Check that all signatures are well formed
    let t = std::time::Instant::now();
    for sig in sigs.values() {
        log::trace!("===== Signature {} =====", &sig.name);
        solver.prove(
            sig.events().chain(sig.params.iter().cloned()),
            sig.constraints.clone(),
            sig.well_formed()?,
            vec![],
        )?;
        log::trace!("==========");
    }
    log::info!("interval-check.sigs: {}ms", t.elapsed().as_millis());

    let mut binds = visitor::Bindings::new(sigs);
    for comp in comps {
        log::trace!("===== Component {} =====", &comp.sig.name);
        Context::check_component(&mut solver, &comp, &binds)?;
        log::trace!("==========");
        // Add the signature of this component to the context.
        binds.add_component(comp);
    }

    ns.components = binds.into();

    log::trace!("Interval checking succeeded");

    Ok(ns)
}
