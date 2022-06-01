use super::{ConcreteInvoke, Context, Fact, THIS};
use crate::{
    core,
    errors::{self, FilamentResult, WithPos},
};
use itertools::Itertools;
use std::collections::HashMap;

// For connect statements of the form:
// dst = src
// The generated proof obligation requires that req(dst) \subsetof guarantees(src)
fn check_connect(con: &core::Connect, ctx: &mut Context) -> FilamentResult<()> {
    let core::Connect {
        dst, src, guard, ..
    } = con;

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
    if let Some(g) = &guard {
        let guard_interval = super::total_interval(g, ctx)?;
        // Require that the guarded value is available for longer that the
        // guard
        if let Some(guarantee) = maybe_guarantee {
            ctx.add_obligation(
                Fact::subset(guard_interval.clone(), guarantee),
                con.copy_span(),
            );
        }
        // Require that the guard availablity satisfies the requirement
        ctx.add_obligation(
            Fact::subset(requirement, guard_interval),
            con.copy_span(),
        );
    } else if let Some(guarantee) = maybe_guarantee {
        ctx.add_obligation(
            Fact::subset(requirement, guarantee),
            con.copy_span(),
        );
    }

    Ok(())
}

/// Check invocation and add new [super::Fact] representing the proof obligations for checking this
/// invocation.
fn check_invocation<'a>(
    invoke: &core::Invocation<super::TimeRep>,
    ctx: &mut Context<'a>,
) -> FilamentResult<ConcreteInvoke<'a>> {
    let sig = ctx.get_instance(&invoke.comp)?;
    let instance =
        ConcreteInvoke::from_signature(sig, invoke.abstract_vars.clone());
    let req_sig = &ctx.get_instance(&invoke.comp)?;
    let req_binding: HashMap<_, _> = req_sig
        .abstract_vars
        .iter()
        .cloned()
        .zip(invoke.abstract_vars.iter().cloned())
        .collect();

    // Add requirements on abstract variables
    req_sig.constraints.iter().for_each(|con| {
        ctx.add_obligation(
            Fact::Constraint(con.resolve(&req_binding)),
            invoke.copy_span(),
        )
    });

    // Add requirements on input ports
    for (actual, formal) in invoke.ports.iter().zip(req_sig.inputs.iter()) {
        // Get requirements for this port
        let requirement = formal.liveness.resolve(&req_binding);
        // Get guarantee for this port
        let maybe_guarantee = match actual {
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
        if let Some(guarantee) = maybe_guarantee {
            let fact = match requirement.typ {
                core::ITag::Exact => Fact::equality(requirement, guarantee),
                core::ITag::Within => Fact::subset(requirement, guarantee),
            };
            ctx.add_obligation(fact, invoke.copy_span());
        }
    }
    Ok(instance)
}

/// Given a [core::Assignment], checks whether the current set of known
/// facts can be used to prove that the ports are available for the stated
/// requirements.
fn check_invoke(
    assign: &core::Invoke<super::TimeRep>,
    ctx: &mut Context,
) -> FilamentResult<()> {
    let instance = check_invocation(&assign.rhs, ctx)?;
    ctx.add_invocation(assign.bind.clone(), instance)?;
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
        log::info!("{}", cmd);
        match cmd {
            core::Command::Invoke(invoke) => check_invoke(invoke, ctx)
                .map_err(|err| err.with_pos(invoke.copy_span()))?,
            core::Command::Instance(core::Instance { name, component }) => {
                ctx.add_instance(name.clone(), component)?
            }
            core::Command::When(wh) => check_when(wh, ctx)?,
            core::Command::Connect(con) => check_connect(con, ctx)
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
    rev_sig
        .constraints
        .iter()
        .for_each(|con| ctx.add_fact(Fact::Constraint(con.clone()), None));

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
