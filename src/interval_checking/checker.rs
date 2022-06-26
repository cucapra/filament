use super::{ConcreteInvoke, Context, THIS};
use crate::core::TimeRep;
use crate::errors::{self, Error, FilamentResult, WithPos};
use crate::event_checker::ast::{self, Constraint};
use itertools::Itertools;
use std::collections::HashMap;

// For connect statements of the form:
// dst = src
// The generated proof obligation requires that req(dst) \subsetof guarantees(src)
fn check_connect(
    dst: &ast::Port,
    src: &ast::Port,
    guard: &Option<ast::Guard>,
    pos: Option<errors::Span>,
    ctx: &mut Context,
) -> FilamentResult<()> {
    ctx.remove_remaning_assign(dst)?;

    let maybe_requirement = ctx.port_requirements(dst)?;
    // If the port does not have any requirement then we don't have any proofs to discharge.
    let requirement = if let Some(req) = maybe_requirement {
        log::debug!("Dest requirement: {req}");
        req
    } else {
        return Ok(());
    };

    let maybe_guarantee = if let Some(g) = ctx.port_guarantees(src)? {
        g
    } else {
        return Err(Error::malformed("Source port does not provide any guarantees and cannot be used to fufill requirements of destination port"));
    };

    // If a guard is present, use its availablity instead.
    let maybe_guarantee = if let Some(g) = &guard {
        let guard_interval = super::total_interval(g, ctx)?;
        log::debug!("Guard availablity is: {guard_interval}");

        // When we have: dst = g ? ...
        // We need to show that:
        // 1. @exact(g) \subsetof @within(dst): To ensure that the guarded signal is keeping a
        //    meaningful value high.
        // 2. @within(dst) \subsetof @within(g): To ensure that the guard is disabling the signal
        //    for long enough.
        if let Some(guarantee) = maybe_guarantee {
            log::debug!("Source guarantee: {guarantee}");
            // Require that the guarded value is available for longer that the
            // guard
            let exact = guard_interval
                .exact
                .as_ref()
                .unwrap_or_else(|| {
                    panic!("Guard signal must have exact specification")
                })
                .clone();
            ctx.add_obligations(
                Constraint::subset(exact, guarantee.within.clone()),
                pos.clone(),
            );

            // Require that the guard's availability is at least as long as the signal.
            ctx.add_obligations(
                Constraint::subset(
                    guarantee.within,
                    guard_interval.within.clone(),
                ),
                pos.clone(),
            );
        }

        Some(guard_interval)
    } else {
        maybe_guarantee
    };

    // If we have: dst = src. We need:
    // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
    // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
    if let Some(guarantee) = &maybe_guarantee {
        let within_fact =
            Constraint::subset(requirement.within, guarantee.within.clone());
        ctx.add_obligations(within_fact, pos.clone());
    }

    if let Some(exact_requirement) = requirement.exact {
        let guarantee = maybe_guarantee.ok_or_else(|| {
            errors::Error::malformed(
                "Constant port cannot provide @exact guarantee",
            )
        })?;

        if let Some(exact_guarantee) = guarantee.exact {
            ctx.add_obligations(
                Constraint::equality(exact_requirement, exact_guarantee),
                pos,
            );
        } else {
            return Err(errors::Error::malformed(
                "Souce port does not provide an exact guarantee while destination port requires it.",
            ));
        }
    }

    Ok(())
}

/// Check invocation and add new [super::Fact] representing the proof obligations for checking this
/// invocation.
fn check_invoke<'a>(
    invoke: &'a ast::Invoke,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    let sig = ctx.get_instance(&invoke.comp)?;
    let binding: HashMap<_, _> = sig
        .abstract_vars
        .iter()
        .cloned()
        .zip(invoke.abstract_vars.iter())
        .collect();

    // Handle `where` clause constraints and well formedness constraints on intervals.
    sig.well_formed()
        // XXX(rachit): This cloned call is stupid
        .chain(sig.constraints.iter().cloned())
        .for_each(|con| {
            ctx.add_obligations(
                Constraint::constraint(con.resolve(&binding)),
                invoke.copy_span(),
            )
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
            let dst = ast::Port::CompPort {
                comp: invoke.bind.clone(),
                name: formal.name.clone(),
            };
            log::info!("checking: {} = {}", dst, actual);
            check_connect(&dst, actual, &None, invoke.copy_span(), ctx)?;
        }
    } else {
        ctx.add_remaning_assigns(invoke.bind.clone(), &invoke.comp)?;
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
        Some(Some(g)) => Ok(g),
        Some(None) => Err(Error::malformed(
            "Constant ports cannot be used to trigger fsm",
        )),
        None => Err(Error::malformed(
            "Port provides no guarantees and cannot be used as fsm trigger",
        )),
    }?;

    let mb_offset = guarantee.as_exact_offset();
    let (ev, start, end) = if let Some(offset) = mb_offset {
        offset
    } else {
        return Err(Error::malformed(
            "Port does not provide an exact guarantee",
        ));
    };
    if end != start + 1 {
        return Err(Error::malformed("Signal is high for too long"));
    }

    // Prove that the signal is zero during the execution of the FSM
    let start_time = crate::core::FsmIdxs::unit(ev.clone(), start);
    let end_time = start_time.clone().increment(*states);
    let within = ast::Range::new(start_time.clone(), end_time);
    ctx.add_obligations(
        ast::Constraint::subset(within, guarantee.within),
        None,
    );

    // Add the FSM instance to the context
    ctx.add_invocation(
        bind.clone(),
        ConcreteInvoke::fsm_instance(start_time, fsm),
    )
}

fn check_commands<'a>(
    cmds: &'a [ast::Command],
    ctx: &mut Context<'a>,
) -> FilamentResult<()>
where
{
    for cmd in cmds {
        log::info!("{cmd}");
        match cmd {
            ast::Command::Invoke(invoke) => check_invoke(invoke, ctx)
                .map_err(|err| err.with_pos(invoke.copy_span()))?,
            ast::Command::Instance(ast::Instance { name, component }) => {
                ctx.add_instance(name.clone(), component)?
            }
            ast::Command::Fsm(fsm) => check_fsm(fsm, ctx)
                .map_err(|err| err.with_pos(fsm.copy_span()))?,
            ast::Command::Connect(
                con @ ast::Connect {
                    dst, src, guard, ..
                },
            ) => check_connect(dst, src, guard, con.copy_span(), ctx)
                .map_err(|err| err.with_pos(con.copy_span()))?,
        };
    }
    Ok(())
}

fn check_component(
    comp: &ast::Component,
    sigs: &HashMap<ast::Id, &ast::Signature>,
) -> FilamentResult<()> {
    let mut ctx = Context::from(sigs);

    // Add instance for this component. Whenever a bare port is used, it refers
    // to the port on this instance.
    let rev_sig = comp.sig.reversed();
    let this_instance = ConcreteInvoke::this_instance(&rev_sig);
    ctx.add_invocation(THIS.into(), this_instance)?;

    // Add constraints on the interface as assumptions
    rev_sig.constraints.iter().for_each(|con| {
        ctx.add_fact(Constraint::constraint(con.clone()), None)
    });

    check_commands(&comp.body, &mut ctx)?;

    // There should be no remaining assignments after checking a component
    if let Some((comp, ports)) = ctx.get_remaining_assigns().next() {
        return Err(Error::malformed(format!(
            "Assignment for invoke missing: {}.{}",
            comp,
            ports.iter().next().unwrap()
        )));
    }

    let (obligations_with_pos, facts) = ctx.into();
    let facts = facts.iter().map(|(f, _)| f).collect_vec();
    /* if !facts.is_empty() {
        eprintln!("Known Facts:\n{:?}", facts);
    } */

    let obligations = obligations_with_pos
        .iter()
        .flat_map(|(f, _)| f.simplify())
        .collect::<Vec<&_>>();
    // eprintln!("Proof Obligations:\n{}", obligations);

    if let Some(fact) = super::prove(
        comp.sig.abstract_vars.iter(),
        facts.into_iter(),
        obligations,
    )? {
        let pos = &obligations_with_pos[fact];
        let err = Err(errors::Error::cannot_prove(fact.clone()));
        if let Some(pos) = pos.get(0) {
            return err.map_err(|err| err.with_pos(Some(pos.clone())));
        } else {
            return err;
        }
    } else {
        eprintln!("All proof obligations satisfied");
    }

    Ok(())
}

/// Check a [ast::Namespace] to prove that the interval requirements of all the ports can be
/// satisfied.
/// Internally generates [super::Fact] which represent proof obligations that need to be proven for
/// the interval requirements to be proven.
pub fn check(namespace: &ast::Namespace) -> FilamentResult<()> {
    let mut sigs = namespace
        .externs
        .iter()
        .flat_map(|(_, comps)| comps.iter().map(|s| (s.name.clone(), s)))
        .collect::<HashMap<_, _>>();

    for comp in &namespace.components {
        log::info!("component {}", comp.sig.name);
        check_component(comp, &sigs)?;
        // Add the signature of this component to the context.
        sigs.insert(comp.sig.name.clone(), &comp.sig);
    }

    Ok(())
}
