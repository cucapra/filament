use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    rc::Rc,
};

use super::Fact;
use crate::{
    core,
    errors::{Error, FilamentResult},
};

/// Representation of a concrete invocation in the context
#[derive(Debug)]
pub struct ConcreteInvoke {
    /// Bindings for abstract variables
    pub binding: HashMap<core::Id, core::IntervalTime>,

    /// Input ports
    pub sig: Rc<core::Signature>,
}

impl ConcreteInvoke {
    /// Construct an instance from a Signature and bindings for abstract variables.
    pub fn from_signature(
        sig: Rc<core::Signature>,
        abs: Vec<core::IntervalTime>,
    ) -> Self {
        let binding = sig
            .abstract_vars
            .iter()
            .cloned()
            .zip(abs.into_iter())
            .collect();

        ConcreteInvoke {
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

        ConcreteInvoke {
            binding,
            sig: Rc::new(sig.reversed()),
        }
    }

    /// Resolve a port for this instance and return the requirement or guarantee
    /// based on whether it is an input or an input port.
    ///
    #[inline]
    fn resolve_port(
        &self,
        port: &core::Id,
        is_input: bool,
    ) -> FilamentResult<core::Interval> {
        let maybe_pd = if is_input {
            self.sig.inputs.iter().find(|pd| pd.name == port)
        } else {
            self.sig.outputs.iter().find(|pd| pd.name == port)
        };
        Ok(maybe_pd
            .ok_or_else(|| {
                let kind = if is_input {
                    "input port"
                } else {
                    "output port"
                };
                Error::Undefined(port.clone(), kind.to_string())
            })?
            .liveness
            .resolve(&self.binding))
    }

    /// Returns the requirements of an input port.
    pub fn port_requirements(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Interval> {
        self.resolve_port(port, true)
    }

    /// Returns the guarantees provided by an output port
    pub fn port_guarantees(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Interval> {
        self.resolve_port(port, false)
    }
}

#[derive(Default, Debug)]
pub struct Context {
    /// Mapping from names to signatures for components and externals.
    sigs: HashMap<core::Id, Rc<core::Signature>>,

    /// Mapping for the names of active instances
    instances: HashMap<core::Id, Rc<core::Signature>>,

    /// Mapping from name of invocations to their information
    invocations: HashMap<core::Id, ConcreteInvoke>,

    /// Set of facts that need to be proven.
    obligations: HashSet<Fact>,

    /// Set of currently known facts.
    facts: HashSet<Fact>,
}

/// Decompose Context into obligations and facts
impl From<Context> for (HashSet<Fact>, HashSet<Fact>) {
    fn from(val: Context) -> Self {
        (val.obligations, val.facts)
    }
}

impl Context {
    /// Add a new definition to the context
    pub fn add_definition(
        &mut self,
        name: core::Id,
        sig: Rc<core::Signature>,
    ) -> FilamentResult<()> {
        if let Entry::Vacant(e) = self.sigs.entry(name.clone()) {
            e.insert(sig);
            Ok(())
        } else {
            Err(Error::AlreadyBound(name, "definition".to_string()))
        }
    }

    /// Add a new instance to the context with the signatuer from `comp`
    pub fn add_instance(
        &mut self,
        name: core::Id,
        comp: &core::Id,
    ) -> FilamentResult<()> {
        match self.sigs.get(comp).map(Rc::clone) {
            Some(sig) => {
                self.instances.insert(name, sig);
                Ok(())
            }
            None => {
                Err(Error::Undefined(comp.clone(), "component".to_string()))
            }
        }
    }

    /// Add a new invocation to the context
    pub fn add_invocation(
        &mut self,
        name: core::Id,
        instance: ConcreteInvoke,
    ) -> FilamentResult<()> {
        if let Entry::Vacant(e) = self.invocations.entry(name.clone()) {
            e.insert(instance);
            Ok(())
        } else {
            Err(Error::AlreadyBound(name, "invocation".to_string()))
        }
    }

    /// Add a new obligation that needs to be proved
    pub fn add_obligation(&mut self, fact: Fact) {
        log::info!("adding obligation {:?}", fact);
        self.obligations.insert(fact);
    }

    /// Get the signature of the component associated with `comp`.
    pub fn get_sig(
        &self,
        comp: &core::Id,
    ) -> FilamentResult<Rc<core::Signature>> {
        self.sigs.get(comp).map(Rc::clone).ok_or_else(|| {
            Error::Undefined(comp.clone(), "component".to_string())
        })
    }

    /// Get the signature of the instance associated with `inst`
    pub fn get_instance(
        &self,
        inst: &core::Id,
    ) -> FilamentResult<Rc<core::Signature>> {
        self.instances.get(inst).map(Rc::clone).ok_or_else(|| {
            Error::Undefined(inst.clone(), "instance".to_string())
        })
    }

    /// Get the instance associated with `instance`
    pub fn get_invoke(
        &self,
        instance: &core::Id,
    ) -> FilamentResult<&ConcreteInvoke> {
        self.invocations.get(instance).ok_or_else(|| {
            Error::Undefined(instance.clone(), "invocation".to_string())
        })
    }
}
