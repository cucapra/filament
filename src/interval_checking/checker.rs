use super::{ConcreteInvoke, Context, THIS};
use crate::{
    core::{self, Constraint},
    errors::{self, FilamentResult, WithPos},
};
use itertools::Itertools;
use std::collections::HashMap;

// For connect statements of the form:
// dst = src
// The generated proof obligation requires that req(dst) \subsetof guarantees(src)
fn check_connect(
    dst: &core::Port,
    src: &core::Port,
    guard: &Option<core::Guard>,
    pos: Option<errors::Span>,
    ctx: &mut Context,
) -> FilamentResult<()> {
    let requirement = match dst {
        core::Port::ThisPort(name) => {
            ctx.get_invoke(&THIS.into())?.port_requirements(name)?
        }
        core::Port::CompPort { comp, name } => {
            ctx.get_invoke(comp)?.port_requirements(name)?
        }
        core::Port::Constant(_) => {
            unreachable!("destination port cannot be a constant")
        }
    };

    // Get guarantee for this port
    let maybe_guarantee = match src {
        core::Port::Constant(_) => {
            /* Constants do not generate a proof obligation because they are
             * always available. */
            None
        }
        core::Port::ThisPort(port) => {
            Some(ctx.get_invoke(&THIS.into())?.port_guarantees(port)?)
        }
        core::Port::CompPort { comp, name } => {
            Some(ctx.get_invoke(comp)?.port_guarantees(name)?)
        }
    };

    // If a guard is present, use its availablity instead.
    let maybe_guarantee = if let Some(g) = &guard {
        let guard_interval = super::total_interval(g, ctx)?;
        log::info!("Guard availablity is: {guard_interval:?}");

        // When we have: dst = g ? ...
        // We need to show that:
        // 1. @exact(g) \subsetof @within(dst): To ensure that the guarded signal is keeping a
        //    meaningful value high.
        // 2. @within(dst) \subsetof @within(g): To ensure that the guard is disabling the signal
        //    for long enough.
        if let Some(guarantee) = maybe_guarantee {
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
    invoke: &core::Invoke<super::TimeRep>,
    ctx: &mut Context<'a>,
) -> FilamentResult<()> {
    let sig = ctx.get_instance(&invoke.comp)?;
    let instance =
        ConcreteInvoke::from_signature(sig, invoke.abstract_vars.clone());
    // Add this invocation to the context
    ctx.add_invocation(invoke.bind.clone(), instance)?;

    let req_sig = &ctx.get_instance(&invoke.comp)?;
    let req_binding: HashMap<_, _> = req_sig
        .abstract_vars
        .iter()
        .cloned()
        .zip(invoke.abstract_vars.iter().cloned())
        .collect();

    // Add requirements on abstract variables
    req_sig.constraints.iter().for_each(|con| {
        ctx.add_obligations(
            Constraint::constraint(con.resolve(&req_binding)),
            invoke.copy_span(),
        )
    });

    // Check connections implied by the invocation
    for (actual, formal) in invoke.ports.iter().zip(req_sig.inputs.iter()) {
        let dst = core::Port::CompPort {
            comp: invoke.bind.clone(),
            name: formal.name.clone(),
        };
        check_connect(&dst, actual, &None, invoke.copy_span(), ctx)?;
    }

    Ok(())
}

fn check_when(
    when: &core::When<super::TimeRep>,
    ctx: &mut Context,
) -> FilamentResult<()> {
    // TODO: Do something with the time variable for the when block
    check_commands(&when.commands, ctx)
}

fn check_commands(
    cmds: &[core::Command<super::TimeRep>],
    ctx: &mut Context,
) -> FilamentResult<()>
where
{
    for cmd in cmds {
        log::info!("{cmd}");
        match cmd {
            core::Command::Invoke(invoke) => check_invoke(invoke, ctx)
                .map_err(|err| err.with_pos(invoke.copy_span()))?,
            core::Command::Instance(core::Instance { name, component }) => {
                ctx.add_instance(name.clone(), component)?
            }
            core::Command::When(wh) => check_when(wh, ctx)?,
            core::Command::Connect(
                con @ core::Connect {
                    dst, src, guard, ..
                },
            ) => check_connect(dst, src, guard, con.copy_span(), ctx)
                .map_err(|err| err.with_pos(con.copy_span()))?,
        };
    }
    Ok(())
}

fn check_component(
    comp: &core::Component<super::TimeRep>,
    sigs: &HashMap<core::Id, &core::Signature<super::TimeRep>>,
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

    let (obligations_with_pos, facts) = ctx.into();
    let facts = facts.iter().map(|(f, _)| f).collect_vec();
    if !facts.is_empty() {
        println!("Known Facts:\n{:#?}", facts);
    }

    let obligations = obligations_with_pos
        .iter()
        .map(|(f, _)| f)
        .collect::<Vec<&_>>();
    println!("Proof Obligations:\n{:#?}", obligations);

    if let Some(fact) = super::prove(
        comp.sig.abstract_vars.iter(),
        facts.into_iter(),
        obligations.into_iter(),
    )? {
        let pos = &obligations_with_pos[fact];
        let err = Err(errors::Error::cannot_prove(fact.clone()));
        if let Some(pos) = pos.get(0) {
            err.map_err(|err| err.with_pos(Some(pos.clone())))
        } else {
            err
        }
    } else {
        println!("All proof obligations satisfied");
        Ok(())
    }
}

/// Check a [core::Namespace] to prove that the interval requirements of all the ports can be
/// satisfied.
/// Internally generates [super::Fact] which represent proof obligations that need to be proven for
/// the interval requirements to be proven.
pub fn check(
    namespace: &core::Namespace<super::TimeRep>,
) -> FilamentResult<()> {
    // Add signatures to the context
    assert!(
        namespace.components.len() <= 1,
        "NYI: Cannot check multiple components"
    );

    let mut sigs = namespace
        .signatures
        .iter()
        .map(|s| (s.name.clone(), s))
        .collect::<HashMap<_, _>>();

    for comp in &namespace.components {
        log::info!("component {}", comp.sig.name);
        check_component(comp, &sigs)?;
        // Add the signature of this component to the context.
        sigs.insert(comp.sig.name.clone(), &comp.sig);
    }

    Ok(())
}
