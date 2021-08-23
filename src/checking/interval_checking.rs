use std::{collections::HashMap, rc::Rc};

use crate::{
    core,
    errors::{Error, FilamentResult},
};

/// Set of known interval facts and equalities.
#[derive(Default)]
pub struct Facts;

pub struct Instance {
    /// Bindings for abstract variables
    pub binding: HashMap<core::Id, core::IntervalTime>,

    /// Input ports
    pub sig: Rc<core::Signature>,
}

impl Instance {
    /// Construct an instance from a Signature and bindings for abstract variables.
    pub fn from_signature(sig: Rc<core::Signature>, abs: Vec<core::IntervalTime>) -> Self {
        let binding = sig
            .abstract_vars
            .iter()
            .cloned()
            .zip(abs.into_iter())
            .collect();

        Instance {
            binding,
            sig: Rc::clone(&sig),
        }
    }

    /// Construct an instance for "this" component.
    pub fn this_instance(sig: Rc<core::Signature>) -> Self {
        // Binding for this instance is just identity.
        let binding = sig
            .abstract_vars
            .iter()
            .cloned()
            .zip(
                sig.abstract_vars
                    .iter()
                    .map(|v| core::IntervalTime::Abstract(v.clone())),
            )
            .collect();
        Instance {
            binding,
            sig: Rc::clone(&sig),
        }
    }
}

#[derive(Default)]
pub struct Context {
    /// Mapping from names to signatures for components and externals.
    pub sigs: HashMap<core::Id, Rc<core::Signature>>,

    /// Mapping from name of instance to its information.
    pub instances: HashMap<core::Id, Instance>,

    /// Set of currently known facts.
    pub facts: Facts,
}

impl Context {
    pub fn add_sig_alias(&mut self, name: core::Id, comp: &core::Id) -> FilamentResult<()> {
        match self.sigs.get(comp).map(|sig| Rc::clone(sig)) {
            Some(sig) => {
                self.sigs.insert(name, sig);
                Ok(())
            }
            None => Err(Error::Undefined(comp.clone(), "component".to_string())),
        }
    }

    pub fn add_sig(&mut self, name: core::Id, sig: Rc<core::Signature>) -> FilamentResult<()> {
        if self.sigs.contains_key(&name) {
            Err(Error::AlreadyBound(name.clone(), "instance".to_string()))
        } else {
            self.sigs.insert(name, sig);
            Ok(())
        }
    }

    pub fn add_instance(&mut self, name: core::Id, instance: Instance) -> FilamentResult<()> {
        if self.instances.contains_key(&name) {
            Err(Error::AlreadyBound(name.clone(), "instance".to_string()))
        } else {
            self.instances.insert(name, instance);
            Ok(())
        }
    }

    pub fn get_sig(&self, comp: &core::Id) -> FilamentResult<Rc<core::Signature>> {
        self.sigs
            .get(comp)
            .map(|sig| Rc::clone(sig))
            .ok_or_else(|| Error::Undefined(comp.clone(), "component".to_string()))
    }
}

/// Return the port requirements of a instance.
fn get_port_requirements(invoke: core::Invocation, ctx: Context) -> FilamentResult<()> {
    todo!()
}

/// Given a [core::Assignment], checks whether the current set of known
/// facts can be used to prove that the ports are available for the stated
/// requirements.
fn check_assign(assign: core::Assignment, ctx: &mut Context) -> FilamentResult<()> {
    // TODO: Check of the invocation is correct.

    // Construct an instance for this invocation.
    let sig = ctx.get_sig(&assign.rhs.comp)?;
    let instance = Instance::from_signature(sig, assign.rhs.abstract_vars);
    ctx.add_instance(assign.bind, instance)?;
    Ok(())
}

fn check_component(comp: core::Component, ctx: &mut Context) -> FilamentResult<()> {
    let sig = Rc::new(comp.sig);

    // Add instance for this component. Whenever a bare port is used, it refers
    // to the port on this instance.
    let this_instance = Instance::this_instance(Rc::clone(&sig));
    ctx.instances.insert("_this".into(), this_instance);

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

    Ok(())
}
