use itertools::Itertools;

use super::{ConcreteInvoke, Context, FilSolver, THIS};
use crate::core::WithTime;
use crate::errors::{Error, FilamentResult, WithPos};
use crate::event_checker::ast::{self, Constraint, CBT};
use crate::visitor;
use std::iter;

// For connect statements of the form:
// dst = src
// The generated proof obligation requires that req(dst) \subsetof guarantees(src)
fn check_connect(
    dst: &ast::Port,
    src: &ast::Port,
    ctx: &mut Context,
) -> FilamentResult<()> {
    log::trace!("Checking connect: {} = {}", dst, src);
    // Remove dst from remaining ports
    ctx.remove_remaning_assign(dst)?;

    let requirement = ctx.port_requirements(dst)?;
    let guarantee = ctx.port_guarantees(src)?;
    let src_pos = src.copy_span();

    // If we have: dst = src. We need:
    // 1. @within(dst) \subsetof @within(src): To ensure that src drives within for long enough.
    // 2. @exact(src) == @exact(dst): To ensure that `dst` exact guarantee is maintained.
    if let Some(guarantee) = &guarantee {
        let within_fact = CBT::subset(requirement.clone(), guarantee.clone())
            .map(|e| {
                Constraint::from(e)
                    .add_note(
                        format!("Source is available for {}", guarantee),
                        src_pos.clone(),
                    )
                    .add_note(
                        format!("Destination's requirement {}", requirement),
                        dst.copy_span(),
                    )
            });
        ctx.add_obligations(within_fact);
    }

    Ok(())
}

fn check_invoke_binds<'a>(
    invoke: &'a ast::Invoke,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    let binding = ctx
        .get_instance(&invoke.instance)
        .binding(&invoke.abstract_vars);

    // Track event bindings
    ctx.add_event_binds(invoke.instance.clone(), &binding, invoke.copy_span());

    let this_sig = ctx.get_invoke(&THIS.into()).get_sig();
    let mut constraints = vec![];

    // For each event provided in the bining, ensure that the corresponding interface
    // does not pulse more often than the interface allows.
    for (abs, evs) in binding.iter() {
        if let Some(inst_event) =
            ctx.get_instance(&invoke.instance).get_event(abs)
        {
            // Each event in the binding must pulse less often than the interface of the abstract
            // variable.
            let event = &evs.event;
            // Get interface for this event
            let event_interface = this_sig.get_event(event);
            let int_len = inst_event.delay.resolve(&binding);
            let ev_int_len = &event_interface.delay;

            // Generate constraint
            let cons = Constraint::from(ast::CBS::gte(
                ev_int_len.clone(),
                int_len.clone(),
            ))
            .add_note(
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
    ctx.add_obligations(constraints.into_iter());

    Ok(())
}

/// Check invocation and add new [super::Fact] representing the proof obligations for checking this
/// invocation.
fn check_invoke<'a>(
    invoke: &'a ast::Invoke,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    // Check the bindings for abstract variables do not violate @interface
    // requirements
    check_invoke_binds(invoke, ctx)?;
    let sig = ctx.get_instance(&invoke.instance).resolve();
    let binding = sig.binding(&invoke.abstract_vars);

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
        // XXX(rachit): Get rid of this clone
        ConcreteInvoke::concrete(binding, sig),
    );

    Ok(())
}

fn check_invoke_ports<'a>(
    invoke: &'a ast::Invoke,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    let sig = ctx.get_instance(&invoke.instance).resolve();
    // If this is a high-level invoke, check all port requirements
    if let Some(actuals) = &invoke.ports {
        // Check connections implied by the invocation
        for (actual, formal) in actuals.iter().zip(sig.inputs()) {
            let dst = ast::Port::comp(invoke.bind.clone(), formal.name.clone())
                .set_span(formal.copy_span());
            check_connect(&dst, actual, ctx)?;
        }
    } else {
        ctx.add_remaning_assigns(invoke.bind.clone(), &invoke.instance)?;
    }

    Ok(())
}

fn check_commands<'a>(
    cmds: &'a [ast::Command],
    ctx: &mut Context<'a>,
) -> FilamentResult<()>
where
{
    // Walk over the commands and add bindings for all invocations
    for cmd in cmds {
        match cmd {
            ast::Command::Invoke(invoke) => check_invoke(invoke, ctx)?,
            ast::Command::Instance(ast::Instance {
                name,
                component,
                bindings,
                ..
            }) => ctx.add_instance(name.clone(), component, bindings),
            ast::Command::Fsm(_) | ast::Command::Connect(_) => (),
        };
    }

    // Check port availability for all connections
    for cmd in cmds {
        match cmd {
            ast::Command::Invoke(invoke) => check_invoke_ports(invoke, ctx)?,
            ast::Command::Connect(ast::Connect {
                dst, src, guard, ..
            }) => {
                if guard.is_none() {
                    check_connect(dst, src, ctx)?
                }
            }
            ast::Command::Instance(_) | ast::Command::Fsm(_) => (),
        };
    }
    Ok(())
}

fn check_component(
    solver: &mut FilSolver,
    comp: &ast::Component,
    sigs: &visitor::Bindings,
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
                None,
            ));
        } else {
            ctx.add_fact(constraint.clone())
        }
    }

    // Check all the commands
    let t = std::time::Instant::now();
    check_commands(&comp.body, &mut ctx)?;
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
        comp.sig.events(),
        &ctx.facts,
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

/// Check a [ast::Namespace] to prove that the interval requirements of all the ports can be
/// satisfied.
/// Internally generates [super::Fact] which represent proof obligations that need to be proven for
/// the interval requirements to be proven.
pub fn check(mut ns: ast::Namespace) -> FilamentResult<ast::Namespace> {
    let comps = ns.components.drain(..).collect_vec();
    let sigs = ns.signatures();
    let mut solver = FilSolver::new()?;

    // Check that all signatures are well formed
    let t = std::time::Instant::now();
    for sig in sigs.values() {
        log::trace!("===== Signature {} =====", &sig.name);
        solver.prove(
            sig.events(),
            &sig.constraints,
            sig.well_formed()?,
            vec![],
        )?;
        log::trace!("==========");
    }
    log::info!("interval-check.sigs: {}ms", t.elapsed().as_millis());

    let mut binds = visitor::Bindings::new(sigs);
    for comp in comps {
        log::trace!("===== Component {} =====", &comp.sig.name);
        check_component(&mut solver, &comp, &binds)?;
        log::trace!("==========");
        // Add the signature of this component to the context.
        binds.add_component(comp);
    }

    ns.components = binds.into();

    log::trace!("Interval checking succeeded");

    Ok(ns)
}
