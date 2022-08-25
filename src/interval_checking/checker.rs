use super::{ConcreteInvoke, Context, THIS};
use crate::core::{Binding, TimeRep, WithTime};
use crate::errors::{self, Error, FilamentResult, WithPos};
use crate::event_checker::ast::{self, Constraint, CBT};
use std::collections::HashMap;
use std::iter;

// For connect statements of the form:
// dst = src
// The generated proof obligation requires that req(dst) \subsetof guarantees(src)
fn check_connect(
    dst: &ast::Port,
    src: &ast::Port,
    guard: &Option<ast::Guard>,
    ctx: &mut Context,
) -> FilamentResult<()> {
    ctx.remove_remaning_assign(dst)?;

    let requirement = ctx.port_requirements(dst)?;
    let src_guarantee = ctx.port_guarantees(src)?;

    // If a guard is present, use its availablity instead.
    let (guarantee, src_pos) = if let Some(g) = &guard {
        let guard_interval = super::total_interval(g, ctx)?;

        // When we have: dst = g ? ...
        // We need to show that:
        // 1. @exact(g) \subsetof @within(dst): To ensure that the guarded signal is keeping a
        //    meaningful value high.
        // 2. @within(dst) \subsetof @within(g): To ensure that the guard is disabling the signal
        //    for long enough.
        if let Some(guarantee) = src_guarantee {
            // Require that the guarded value is available for longer that the
            // guard
            let guard_exact = guard_interval
                .exact
                .as_ref()
                .ok_or_else(|| {
                    Error::malformed(
                        "Guard signal must have exact specification",
                    )
                    .add_note(
                        format!("Guard's specification is {}", guard_interval),
                        g.copy_span(),
                    )
                })?
                .clone();

            ctx.add_obligations(
                CBT::subset(guard_exact.clone(), guarantee.within.clone()).map(|e| {
                    Constraint::from(e).add_note("Guard's @exact specification must be shorter than source", g.copy_span())
                     .add_note(format!("Guard's exact specification is {}", guard_exact), g.copy_span())
                     .add_note(format!("Source's specification is {}", guarantee), src.copy_span())
                }),
            );

            // Require that the guard's availability is at least as long as the signal.
            ctx.add_obligations(
                CBT::subset(
                    guarantee.within.clone(),
                    guard_interval.within.clone(),
                )
                .map(|e|
                     Constraint::from(e).add_note("Guard must be active for at least as long as the source", g.copy_span())
                      .add_note(format!("Guard is active during {}", guard_interval), g.copy_span())
                      .add_note(format!("Source is active for {}", guarantee), src.copy_span())),
            );
        }

        (Some(guard_interval), g.copy_span())
    } else {
        (src_guarantee, src.copy_span())
    };

    // If we have: dst = src. We need:
    // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
    // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
    if let Some(guarantee) = &guarantee {
        let within_fact =
            CBT::subset(requirement.within.clone(), guarantee.within.clone())
                .map(|e| {
                    Constraint::from(e)
                        .add_note(
                            format!(
                                "Source is available for {}",
                                guarantee
                            ),
                            src_pos.clone(),
                        )
                        .add_note(
                            format!(
                                "Destination's requirement {}",
                                requirement
                            ),
                            dst.copy_span(),
                        )
                });
        ctx.add_obligations(within_fact);
    }

    if let Some(exact_requirement) = requirement.exact {
        let guarantee = guarantee.ok_or_else(|| {
            errors::Error::malformed(
                "Destination requires @exact guarantee but source does not provide it",
            ).add_note("Constant port cannot provide @exact specification", src.copy_span())
        })?;

        if let Some(exact_guarantee) = guarantee.exact {
            ctx.add_obligations(
                CBT::equality(exact_requirement.clone(), exact_guarantee.clone())
                    .map(|e| Constraint::from(e).add_note("Source must satisfy the exact guarantee of the destination", src_pos.clone())
                              .add_note(format!("Source's availablity is {}", exact_guarantee), src_pos.clone())
                              .add_note(format!("Destination's requirement is {}", exact_requirement), dst.copy_span())),
            );
        } else {
            return Err(errors::Error::malformed(
                "Souce port does not provide an exact guarantee while destination port requires it.",
            ));
        }
    }

    Ok(())
}

fn check_invoke_binds<'a>(
    invoke: &'a ast::Invoke,
    ctx: &mut Context<'a>,
) -> FilamentResult<(&'a ast::Signature, Binding<'a, ast::TimeRep>)> {
    // Track event bindings
    ctx.add_event_binds(
        invoke.instance.clone(),
        invoke.abstract_vars.clone(),
        invoke.copy_span(),
    );

    let sig = ctx.get_instance(&invoke.instance)?;
    let binding = sig.binding(&invoke.abstract_vars).map_err(|err| {
        err.add_note(
            format!(
                "Invocation provides {} events",
                invoke.abstract_vars.len()
            ),
            invoke.copy_span(),
        )
    })?;
    let this_sig = ctx.get_invoke(&THIS.into())?.get_sig();
    let mut constraints = vec![];

    // For each event provided for an abstract variable, ensure that the corresponding interface
    // does not pulse more often than the interface allows.
    for (abs, &evs) in binding.iter() {
        if let Some(interface) = sig.get_interface(abs) {
            // Each event in the binding must pulse less often than the interface of the abstract
            // variable.
            for (event, _) in evs.events() {
                // Get interface for this event
                let event_interface =
                    this_sig.get_interface(event).ok_or_else(|| {
                        Error::malformed(format!(
                            "Event {event} does not have a corresponding interface signal"
                        ))
                    })?;
                let int_len = interface.delay().resolve(&binding);
                let ev_int_len = event_interface.delay();

                // Generate constraint
                let cons = Constraint::from(ast::CBS::gte(
                    ev_int_len.clone(),
                    int_len.clone(),
                ))
                .add_note(
                    "Event provided to invoke pulses more often than @interface allows",
                    invoke.copy_span()
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
                        "Interface requires event to trigger once in {} cycles",
                        int_len,
                    ),
                    interface.copy_span(),
                );
                constraints.push(cons);
            }
        }
    }
    ctx.add_obligations(constraints.into_iter());

    Ok((sig, binding))
}

/// Check invocation and add new [super::Fact] representing the proof obligations for checking this
/// invocation.
fn check_invoke<'a>(
    invoke: &'a ast::Invoke,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    // Check the bindings for abstract variables do not violate @interface
    // requirements
    let (sig, binding) = check_invoke_binds(invoke, ctx)?;

    // Handle `where` clause constraints and well formedness constraints on intervals.
    sig.well_formed()?.for_each(|con| {
        ctx.add_obligations(iter::once(con.resolve(&binding)).map(|e| {
            e.add_note(
                "Invoke's intervals must be well-formed",
                invoke.copy_span(),
            )
        }))
    });

    sig.constraints.iter().for_each(|con| {
        ctx.add_obligations(iter::once(con.resolve(&binding)).map(|e| {
            e.add_note(
                "Component's where clause constraints must be satisfied",
                invoke.copy_span(),
            )
        }))
    });

    // Add this invocation to the context
    ctx.add_invocation(
        invoke.bind.clone(),
        ConcreteInvoke::concrete(binding, sig),
    )?;

    // If this is a high-level invoke, check all port requirements
    if let Some(actuals) = &invoke.ports {
        // Check connections implied by the invocation
        for (actual, formal) in actuals.iter().zip(sig.inputs.iter()) {
            let dst = ast::Port::comp(invoke.bind.clone(), formal.name.clone())
                .set_span(formal.copy_span());
            check_connect(&dst, actual, &None, ctx)?;
        }
    } else {
        ctx.add_remaning_assigns(invoke.bind.clone(), &invoke.instance)?;
    }

    Ok(())
}

fn check_fsm<'a>(
    fsm: &'a ast::Fsm,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    let ast::Fsm {
        name: bind,
        states,
        trigger,
        ..
    } = fsm;

    let guarantee = match ctx.port_guarantees(trigger)? {
        Some(g) => Ok(g),
        None => Err(Error::malformed("Invalid port for fsm trigger").add_note(
            "Cannot use constant port to trigger fsm",
            trigger.copy_span(),
        )),
    }?;

    let mb_offset = guarantee.as_exact_offset();
    let (ev, start, end) = if let Some(offset) = mb_offset {
        offset
    } else {
        return Err(Error::malformed(
            "FSMs trigger port must have an @exact specification",
        )
        .add_note(
            format!("Port's specification is {}", guarantee),
            trigger.copy_span(),
        ));
    };
    if end != start + 1 {
        return Err(Error::malformed("Trigger port is high for too long")
            .add_note(
                format!("Trigger port is active for {} cycles", end - start),
                trigger.copy_span(),
            ));
    }

    // Prove that the signal is zero during the execution of the FSM
    let start_time = ast::TimeRep::unit(ev.clone(), start);
    let end_time = start_time.clone().increment(*states);
    let within = ast::Range::new(start_time.clone(), end_time);
    ctx.add_obligations(ast::CBT::subset(within, guarantee.within.clone()).map(|e| {
        Constraint::from(e).add_note(
            "Trigger must not pulse more often than the FSM states",
            trigger.copy_span(),
        )
    }));

    // Add the FSM instance to the context
    ctx.add_invocation(
        bind.clone(),
        ConcreteInvoke::fsm_instance(guarantee, fsm),
    )
}

fn check_commands<'a>(
    cmds: &'a [ast::Command],
    ctx: &mut Context<'a>,
) -> FilamentResult<()>
where
{
    for cmd in cmds {
        log::trace!("Checking command: {}", cmd);
        match cmd {
            ast::Command::Invoke(invoke) => check_invoke(invoke, ctx)?,
            ast::Command::Instance(ast::Instance { name, component }) => {
                ctx.add_instance(name.clone(), component)?
            }
            ast::Command::Fsm(fsm) => check_fsm(fsm, ctx)?,
            ast::Command::Connect(ast::Connect {
                dst, src, guard, ..
            }) => check_connect(dst, src, guard, ctx)?,
        };
    }
    Ok(())
}

fn check_component(
    comp: &ast::Component,
    sigs: &HashMap<ast::Id, &ast::Signature>,
) -> FilamentResult<()> {
    let mut ctx = Context::from(sigs);

    // Ensure that the signature is well-formed
    ctx.add_obligations(comp.sig.well_formed()?);

    // Add instance for this component. Whenever a bare port is used, it refers
    // to the port on this instance.
    let rev_sig = comp.sig.reversed();
    let this_instance = ConcreteInvoke::this_instance(&rev_sig);
    ctx.add_invocation(THIS.into(), this_instance)?;

    // Add constraints on the interface as assumptions
    ctx.add_facts(rev_sig.constraints.iter().cloned());

    // Check all the commands
    check_commands(&comp.body, &mut ctx)?;
    // Add obligations from disjointness constraints
    let disj = ctx.drain_disjointness()?;
    ctx.add_obligations(disj);

    // Prove all the required obligations
    let obs = ctx.drain_obligations();
    super::prove(comp.sig.abstract_vars.iter(), &ctx.facts, obs)?;

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

/// Check a [ast::Namespace] to prove that the interval requirements of all the ports can be
/// satisfied.
/// Internally generates [super::Fact] which represent proof obligations that need to be proven for
/// the interval requirements to be proven.
pub fn check(namespace: &ast::Namespace) -> FilamentResult<()> {
    let mut sigs = namespace.signatures();

    // Check that all signatures are well formed
    for sig in sigs.values() {
        log::info!("===== Signature {} =====", &sig.name);
        super::prove(
            sig.abstract_vars.iter(),
            &sig.constraints,
            sig.well_formed()?,
        )?;
        log::info!("==========");
    }

    for comp in &namespace.components {
        log::info!("===== Component {} =====", &comp.sig.name);
        check_component(comp, &sigs)?;
        log::info!("==========");
        // Add the signature of this component to the context.
        sigs.insert(comp.sig.name.clone(), &comp.sig);
    }

    log::info!("Interval checking succeeded");

    Ok(())
}
