use std::rc::Rc;

use super::{Context, Fact, Instance};

use crate::{core, errors::FilamentResult};

const THIS: &str = "_this";

// For connect statements of the form:
// dst = src
// The generated proof obligation requires that req(dst) \subsetof guarantees(src)
fn check_connect(con: &core::Connect, ctx: &mut Context) -> FilamentResult<()> {
    let core::Connect { dst, src } = con;
    let requirement = match dst {
        core::Port::ThisPort(name) => {
            ctx.get_instance(&THIS.into())?.port_requirements(name)?
        }
        core::Port::CompPort { comp, name } => {
            ctx.get_instance(comp)?.port_requirements(name)?
        }
        core::Port::Constant(_) => {
            todo!("destination port cannot be a constant")
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
            Some(ctx.get_instance(&THIS.into())?.port_guarantees(port)?)
        }
        core::Port::CompPort { comp, name } => {
            Some(ctx.get_instance(comp)?.port_guarantees(name)?)
        }
    };
    if let Some(guarantee) = maybe_guarantee {
        ctx.obligations.insert(Fact::subset(requirement, guarantee));
    }
    Ok(())
}

/// Check invocation and add new [super::Fact] representing the proof obligations for checking this
/// invocation.
fn check_invocation(
    invoke: &core::Invocation,
    ctx: &mut Context,
) -> FilamentResult<Instance> {
    // Construct an instance for this invocation
    let sig = ctx.get_sig(&invoke.comp)?;
    let instance = Instance::from_signature(sig, invoke.abstract_vars.clone());
    let req_sig = &ctx.get_sig(&invoke.comp)?;
    let req_binding = req_sig
        .abstract_vars
        .iter()
        .cloned()
        .zip(invoke.abstract_vars.iter().cloned())
        .collect();

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
                Some(ctx.get_instance(&THIS.into())?.port_guarantees(port)?)
            }
            core::Port::CompPort { comp, name } => {
                Some(ctx.get_instance(comp)?.port_guarantees(name)?)
            }
        };
        if let Some(guarantee) = maybe_guarantee {
            ctx.obligations.insert(Fact::subset(requirement, guarantee));
        }
    }
    Ok(instance)
}

/// Given a [core::Assignment], checks whether the current set of known
/// facts can be used to prove that the ports are available for the stated
/// requirements.
fn check_assign(
    assign: &core::Invoke,
    ctx: &mut Context,
) -> FilamentResult<()> {
    let instance = check_invocation(&assign.rhs, ctx)?;
    ctx.add_instance(assign.bind.clone(), instance)?;
    Ok(())
}

fn check_when(when: &core::When, ctx: &mut Context) -> FilamentResult<()> {
    // TODO: Do something with the time variable for the when block
    check_commands(&when.commands, ctx)
}

fn check_commands(
    cmds: &[core::Command],
    ctx: &mut Context,
) -> FilamentResult<()>
where
{
    for cmd in cmds {
        match cmd {
            core::Command::Invoke(invoke) => check_assign(invoke, ctx)?,
            core::Command::Instance(core::Cell { name, component }) => {
                ctx.add_sig_alias(name.clone(), component)?
            }
            core::Command::When(wh) => check_when(wh, ctx)?,
            core::Command::Connect(con) => check_connect(con, ctx)?,
        };
    }
    Ok(())
}

fn check_component(
    comp: core::Component,
    ctx: &mut Context,
) -> FilamentResult<()> {
    let sig = Rc::new(comp.sig);

    // Add instance for this component. Whenever a bare port is used, it refers
    // to the port on this instance.
    let this_instance = Instance::this_instance(Rc::clone(&sig));
    ctx.instances.insert(THIS.into(), this_instance);

    check_commands(&comp.body, ctx)?;

    Ok(())
}

/// Check a [core::Namespace] to prove that the interval requirements of all the ports can be
/// satisfied.
/// Internally generates [super::Fact] which represent proof obligations that need to be proven for
/// the interval requirements to be proven.
pub fn check(mut namespace: core::Namespace) -> FilamentResult<()> {
    let mut ctx = Context::default();
    // Add signatures to the context
    namespace.signatures.drain(..).try_for_each(|sig| {
        let name = sig.name.clone();
        let sig_ref = Rc::new(sig);
        ctx.add_sig(name, sig_ref)
    })?;

    assert!(
        namespace.components.len() <= 1,
        "NYI: Cannot check multiple components"
    );

    namespace
        .components
        .drain(..)
        .try_for_each(|comp| check_component(comp, &mut ctx))?;

    println!("Known Facts:\n{:#?}", ctx.facts);
    println!("Proof Obligations:\n{:#?}", ctx.obligations);

    Ok(())
}
