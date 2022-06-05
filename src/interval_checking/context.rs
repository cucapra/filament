use std::collections::{hash_map::Entry, HashMap, HashSet};

use linked_hash_map::LinkedHashMap;

use super::TimeRep;
use crate::{
    core,
    errors::{self, Error, FilamentResult},
};

pub enum ConcreteInvoke<'a> {
    CI {
        /// Bindings for abstract variables
        binding: HashMap<core::Id, super::TimeRep>,

        /// Input ports
        sig: &'a core::Signature<TimeRep>,
    },
    Fsm {
        /// Start time of this FSM
        start_time: super::TimeRep,
        /// Internal FSM
        fsm: &'a core::Fsm,
    },
}

impl<'a> ConcreteInvoke<'a> {
    /// Construct an instance from a Signature and bindings for abstract variables.
    pub fn from_signature(
        sig: &'a core::Signature<TimeRep>,
        abs: Vec<super::TimeRep>,
    ) -> Self {
        let binding = sig
            .abstract_vars
            .iter()
            .cloned()
            .zip(abs.into_iter())
            .collect();

        Self::CI { binding, sig }
    }

    /// Construct an instance for "this" component.
    pub fn this_instance(sig: &'a core::Signature<TimeRep>) -> Self {
        // Binding for this instance is just identity.
        let binding = sig
            .abstract_vars
            .iter()
            .cloned()
            .zip(
                sig.abstract_vars
                    .iter()
                    .map(|v| core::FsmIdxs::unit(v.clone(), 0)),
            )
            .collect();

        Self::CI { binding, sig }
    }

    /// Construct an FSM instance
    pub fn fsm_instance(
        start_time: super::TimeRep,
        fsm: &'a core::Fsm,
    ) -> Self {
        Self::Fsm { start_time, fsm }
    }

    /// Resolve a port for this instance and return the requirement or guarantee
    /// based on whether it is an input or an input port.
    #[inline]
    fn resolve_port(
        &self,
        port: &core::Id,
        is_input: bool,
    ) -> FilamentResult<core::Interval<TimeRep>> {
        match self {
            ConcreteInvoke::CI { binding, sig } => {
                let pd = sig.get_port(port, is_input)?;
                Ok(pd.liveness.resolve(binding))
            }
            ConcreteInvoke::Fsm { start_time, fsm } => {
                // XXX(rachit): This is constructed everytime this method is called.
                let idx = fsm.state(port)?;
                let within: core::Range<super::TimeRep> = core::Range::new(
                    start_time.clone(),
                    start_time.clone().increment(fsm.states),
                );
                let exact = core::Range::new(
                    start_time.clone().increment(idx),
                    start_time.clone().increment(idx + 1),
                );
                Ok(core::Interval::from(within).with_exact(exact))
            }
        }
    }

    /// Returns the requirements of an input port.
    pub fn port_requirements(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Interval<TimeRep>> {
        self.resolve_port(port, true)
    }

    /// Returns the guarantees provided by an output port
    pub fn port_guarantees(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Interval<TimeRep>> {
        self.resolve_port(port, false)
    }
}

type FactMap = LinkedHashMap<core::Constraint<TimeRep>, Vec<errors::Span>>;

pub struct Context<'a> {
    /// Mapping from names to signatures for components and externals.
    sigs: &'a HashMap<core::Id, &'a core::Signature<TimeRep>>,

    /// Mapping for the names of active instances
    instances: HashMap<core::Id, &'a core::Signature<TimeRep>>,

    /// Mapping from name of invocations to their information
    invocations: HashMap<core::Id, ConcreteInvoke<'a>>,

    /// Remaining assigmments for a given invoke.
    remaining_assigns: HashMap<core::Id, HashSet<core::Id>>,

    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    obligations: FactMap,

    /// Set of assumed facts
    facts: FactMap,
}

impl<'a> From<&'a HashMap<core::Id, &'a core::Signature<TimeRep>>>
    for Context<'a>
{
    fn from(sigs: &'a HashMap<core::Id, &'a core::Signature<TimeRep>>) -> Self {
        Context {
            sigs,
            remaining_assigns: HashMap::default(),
            instances: HashMap::default(),
            invocations: HashMap::default(),
            obligations: LinkedHashMap::default(),
            facts: LinkedHashMap::default(),
        }
    }
}

/// Decompose Context into obligations and facts
impl From<Context<'_>> for (FactMap, FactMap) {
    fn from(val: Context) -> Self {
        (val.obligations, val.facts)
    }
}

impl<'a> Context<'a> {
    /// Add a new instance to the context with the signatuer from `comp`
    pub fn add_instance(
        &mut self,
        name: core::Id,
        comp: &core::Id,
    ) -> FilamentResult<()> {
        match self.sigs.get(comp) {
            Some(sig) => {
                self.instances.insert(name, sig);
                Ok(())
            }
            None => {
                Err(Error::undefined(comp.clone(), "component".to_string()))
            }
        }
    }

    /// Add a new invocation to the context
    pub fn add_invocation(
        &mut self,
        name: core::Id,
        instance: ConcreteInvoke<'a>,
    ) -> FilamentResult<()> {
        if let Entry::Vacant(e) = self.invocations.entry(name.clone()) {
            e.insert(instance);
            Ok(())
        } else {
            Err(Error::already_bound(name, "invocation".to_string()))
        }
    }

    pub fn add_remaning_assigns(
        &mut self,
        bind: core::Id,
        comp: &core::Id,
    ) -> FilamentResult<()> {
        let sig = self.get_instance(comp)?;
        let ports = sig
            .inputs
            .iter()
            .chain(sig.interface_signals.iter())
            .map(|pd| &pd.name)
            .cloned()
            .collect();
        self.remaining_assigns.insert(bind, ports);
        Ok(())
    }

    pub fn remove_remaning_assign(
        &mut self,
        port: &core::Port,
    ) -> FilamentResult<()> {
        match port {
            core::Port::CompPort { comp, name } => {
                // Check if the port is defined
                self.get_invoke(comp)?.resolve_port(name, true)?;
                if let Some(ports) = self.remaining_assigns.get_mut(comp) {
                    if !ports.remove(name) {
                        return Err(Error::malformed(format!(
                            "Multiple assignments to port: {}",
                            port
                        )));
                    }
                }
                Ok(())
            }
            core::Port::ThisPort(_) | core::Port::Constant(_) => Ok(()),
        }
    }

    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F, span: Option<errors::Span>)
    where
        F: Iterator<Item = core::Constraint<TimeRep>>,
    {
        for fact in facts {
            log::info!("adding obligation {:?}", fact);
            let locs = self.obligations.entry(fact).or_insert(vec![]);
            if let Some(sp) = &span {
                locs.push(sp.clone())
            }
        }
    }

    /// Add a new known fact
    pub fn add_fact<F>(&mut self, facts: F, span: Option<errors::Span>)
    where
        F: Iterator<Item = core::Constraint<TimeRep>>,
    {
        for fact in facts {
            log::info!("adding known fact {:?}", fact);
            let locs = self.facts.entry(fact).or_insert(vec![]);
            if let Some(sp) = &span {
                locs.push(sp.clone())
            }
        }
    }

    /// Get the signature of the instance associated with `inst`
    pub fn get_instance(
        &self,
        inst: &core::Id,
    ) -> FilamentResult<&'a core::Signature<TimeRep>> {
        self.instances.get(inst).copied().ok_or_else(|| {
            Error::undefined(inst.clone(), "instance".to_string())
        })
    }

    /// Get the instance associated with `instance`
    pub fn get_invoke(
        &self,
        instance: &core::Id,
    ) -> FilamentResult<&ConcreteInvoke> {
        self.invocations.get(instance).ok_or_else(|| {
            Error::undefined(instance.clone(), "invocation".to_string())
        })
    }

    /// Return the remaining assignments in this context
    pub fn get_remaining_assigns(
        &self,
    ) -> impl Iterator<Item = (&core::Id, &HashSet<core::Id>)> {
        self.remaining_assigns
            .iter()
            .filter(|(_, ports)| !ports.is_empty())
    }

    pub fn port_guarantees(
        &self,
        port: &core::Port,
    ) -> FilamentResult<Option<core::Interval<super::TimeRep>>> {
        match port {
            core::Port::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                Ok(None)
            }
            core::Port::ThisPort(port) => Ok(Some(
                self.get_invoke(&super::THIS.into())?
                    .port_guarantees(port)?,
            )),
            core::Port::CompPort { comp, name } => {
                Ok(Some(self.get_invoke(comp)?.port_guarantees(name)?))
            }
        }
    }

    pub fn port_requirements(
        &self,
        port: &core::Port,
    ) -> FilamentResult<core::Interval<super::TimeRep>> {
        match port {
            core::Port::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                unreachable!("destination port cannot be a constant")
            }
            core::Port::ThisPort(port) => Ok(self
                .get_invoke(&super::THIS.into())?
                .port_requirements(port)?),
            core::Port::CompPort { comp, name } => {
                Ok(self.get_invoke(comp)?.port_requirements(name)?)
            }
        }
    }
}
