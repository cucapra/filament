use std::{
    collections::{hash_map::Entry, HashMap},
    rc::Rc,
};

use crate::{
    core,
    errors::{Error, FilamentResult},
};

const THIS: &str = "_this";

pub enum FactType {
    Equality,
    Subset,
}

/// Set of known interval facts and equalities.
pub struct Fact {
    tag: FactType,
    left: core::Interval,
    right: core::Interval,
}

#[derive(Debug)]
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

    /// Resolve a port for this instance and return the requirement or guarantee
    /// based on whether it is an input or an input port.
    ///
    #[inline]
    fn resolve_port(&self, port: &core::Id, is_input: bool) -> FilamentResult<core::Interval> {
        let maybe_pd = if is_input {
            self.sig.inputs.iter().find(|pd| pd.name == port)
        } else {
            self.sig.outputs.iter().find(|pd| pd.name == port)
        };
        Ok(maybe_pd
            .ok_or_else(|| Error::Undefined(port.clone(), "port".to_string()))?
            .liveness
            .resolve(&self.binding))
    }

    /// Returns the requirements of an input port
    pub fn port_requirements(&self, port: &core::Id) -> FilamentResult<core::Interval> {
        self.resolve_port(port, true)
    }

    /// Returns the guarantees provided by an output port
    pub fn port_guarantees(&self, port: &core::Id) -> FilamentResult<core::Interval> {
        self.resolve_port(port, false)
    }
}

#[derive(Default)]
pub struct Context {
    /// Mapping from names to signatures for components and externals.
    pub sigs: HashMap<core::Id, Rc<core::Signature>>,

    /// Mapping from name of instance to its information.
    pub instances: HashMap<core::Id, Instance>,

    /// Set of currently known facts.
    pub facts: Vec<Fact>,
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
        if let Entry::Vacant(e) = self.sigs.entry(name.clone()) {
            e.insert(sig);
            Ok(())
        } else {
            Err(Error::AlreadyBound(name, "instance".to_string()))
        }
    }

    pub fn add_instance(&mut self, name: core::Id, instance: Instance) -> FilamentResult<()> {
        if let Entry::Vacant(e) = self.instances.entry(name.clone()) {
            e.insert(instance);
            Ok(())
        } else {
            Err(Error::AlreadyBound(name, "instance".to_string()))
        }
    }

    pub fn get_sig(&self, comp: &core::Id) -> FilamentResult<Rc<core::Signature>> {
        self.sigs
            .get(comp)
            .map(|sig| Rc::clone(sig))
            .ok_or_else(|| Error::Undefined(comp.clone(), "component".to_string()))
    }

    pub fn get_instance(&self, instance: &core::Id) -> FilamentResult<&Instance> {
        self.instances
            .get(instance)
            .ok_or_else(|| Error::Undefined(instance.clone(), "instance".to_string()))
    }
}

/// Check invocation and add new [Fact] representing the proof obligations for checking this
/// invocation.
fn check_invocation(invoke: core::Invocation, ctx: &mut Context) -> FilamentResult<Instance> {
    // Construct an instance for this invocation
    let sig = ctx.get_sig(&invoke.comp)?;
    let instance = Instance::from_signature(sig, invoke.abstract_vars);
    for port in invoke.ports {
        // Get guarantee for this port
        let guarantee = match port {
            core::Port::Constant(_) => {
                /* Constants do not generate a proof obligation */
                None
            }
            core::Port::ThisPort(port) => {
                Some(ctx.get_instance(&THIS.into())?.port_guarantees(&port)?)
            }
            core::Port::CompPort { comp, name } => {
                Some(ctx.get_instance(&comp)?.port_guarantees(&name)?)
            }
        };
        // Get requirements for this port
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

    println!("{:#?}", ctx.instances);

    Ok(())
}
