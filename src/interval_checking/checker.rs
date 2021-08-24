use std::rc::Rc;

use super::{Context, Fact, Instance};

use crate::{core, errors::FilamentResult};

const THIS: &str = "_this";

/// Check invocation and add new [Fact] representing the proof obligations for checking this
/// invocation.
fn check_invocation(invoke: core::Invocation, ctx: &mut Context) -> FilamentResult<Instance> {
    // Construct an instance for this invocation
    let sig = ctx.get_sig(&invoke.comp)?;
    let instance = Instance::from_signature(sig, invoke.abstract_vars.clone());
    let req_sig = &ctx.get_sig(&invoke.comp)?;
    let req_binding = req_sig
        .abstract_vars
        .iter()
        .cloned()
        .zip(invoke.abstract_vars.into_iter())
        .collect();
    for (actual, formal) in invoke.ports.iter().zip(req_sig.inputs.iter()) {
        // Get requirements for this port
        let requirement = formal.liveness.resolve(&req_binding);
        // Get guarantee for this port
        let maybe_guarantee = match actual {
            core::Port::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always avaiable. */
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
            let fact = match requirement.tag {
                core::IntervalType::Exact => Fact::equality(requirement, guarantee),
                core::IntervalType::Within => Fact::subset(requirement, guarantee),
            };
            ctx.facts.insert(fact);
        }
    }
    Ok(instance)
}

/// Given a [core::Assignment], checks whether the current set of known
/// facts can be used to prove that the ports are available for the stated
/// requirements.
fn check_assign(assign: core::Assignment, ctx: &mut Context) -> FilamentResult<()> {
    let instance = check_invocation(assign.rhs, ctx)?;
    ctx.add_instance(assign.bind, instance)?;
    Ok(())
}

fn check_component(comp: core::Component, ctx: &mut Context) -> FilamentResult<()> {
    let sig = Rc::new(comp.sig);

    // Add instance for this component. Whenever a bare port is used, it refers
    // to the port on this instance.
    let this_instance = Instance::this_instance(Rc::clone(&sig));
    ctx.instances.insert(THIS.into(), this_instance);

    // Add aliases to components for all cells.
    for cell in comp.cells {
        ctx.add_sig_alias(cell.name, &cell.component)?;
    }

    comp.assignments
        .into_iter()
        .try_for_each(|assign| check_assign(assign, ctx))?;

    Ok(())
}

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

    println!("Proof Obligations:\n{:#?}", ctx.facts);

    Ok(())
}
