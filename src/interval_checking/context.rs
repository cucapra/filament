use std::{collections::{HashMap, HashSet, hash_map::Entry}, rc::Rc};

use super::Fact;
use crate::{core, errors::{Error, FilamentResult}};

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
    /* pub fn port_requirements(&self, port: &core::Id) -> FilamentResult<core::Interval> {
        self.resolve_port(port, true)
    } */

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
    pub facts: HashSet<Fact>,
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

