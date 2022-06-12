use std::collections::{hash_map::Entry, HashMap, HashSet};

use linked_hash_map::LinkedHashMap;

use crate::core::FsmIdxs;
use crate::errors::{self, Error, FilamentResult};
use crate::event_checker::ast;

pub enum ConcreteInvoke<'a> {
    Concrete {
        /// Bindings for abstract variables
        binding: HashMap<ast::Id, &'a FsmIdxs>,

        /// Signature
        sig: &'a ast::Signature,
    },
    Fsm {
        /// Start time of this FSM
        start_time: FsmIdxs,
        /// Internal FSM
        fsm: &'a ast::Fsm,
    },
    This {
        /// Signature
        sig: &'a ast::Signature,
    },
}

impl<'a> ConcreteInvoke<'a> {
    /// Construct an instance from a Signature and bindings for abstract variables.
    pub fn concrete(
        binding: HashMap<ast::Id, &'a FsmIdxs>,
        sig: &'a ast::Signature,
    ) -> Self {
        Self::Concrete { binding, sig }
    }

    /// Construct an instance for "this" component.
    pub fn this_instance(sig: &'a ast::Signature) -> Self {
        Self::This { sig }
    }

    /// Construct an FSM instance
    pub fn fsm_instance(start_time: FsmIdxs, fsm: &'a ast::Fsm) -> Self {
        Self::Fsm { start_time, fsm }
    }

    /// Resolve a port for this instance and return the requirement or guarantee
    /// based on whether it is an input or an input port.
    #[inline]
    fn resolve_port(
        &self,
        port: &ast::Id,
        is_input: bool,
    ) -> FilamentResult<Option<ast::Interval>> {
        match self {
            ConcreteInvoke::Concrete { binding, sig } => {
                let pd = sig.get_port(port, is_input)?;
                Ok(pd.liveness.as_ref().map(|l| l.resolve(binding)))
            }
            ConcreteInvoke::Fsm { start_time, fsm } => {
                // XXX(rachit): This is constructed everytime this method is called.
                let idx = fsm.state(port)?;
                let within: ast::Range = ast::Range::new(
                    start_time.clone(),
                    start_time.clone().increment(fsm.states),
                );
                let exact = ast::Range::new(
                    start_time.clone().increment(idx),
                    start_time.clone().increment(idx + 1),
                );
                Ok(Some(ast::Interval::from(within).with_exact(exact)))
            }
            ConcreteInvoke::This { sig } => {
                Ok(sig.get_port(port, is_input)?.liveness.clone())
            }
        }
    }

    /// Returns the requirements of an input port.
    pub fn port_requirements(
        &self,
        port: &ast::Id,
    ) -> FilamentResult<Option<ast::Interval>> {
        self.resolve_port(port, true)
    }

    /// Returns the guarantees provided by an output port
    pub fn port_guarantees(
        &self,
        port: &ast::Id,
    ) -> FilamentResult<Option<ast::Interval>> {
        self.resolve_port(port, false)
    }
}

type FactMap = LinkedHashMap<ast::Constraint, Vec<errors::Span>>;

pub struct Context<'a> {
    /// Mapping from names to signatures for components and externals.
    sigs: &'a HashMap<ast::Id, &'a ast::Signature>,

    /// Mapping for the names of active instances
    instances: HashMap<ast::Id, &'a ast::Signature>,

    /// Mapping from name of invocations to their information
    invocations: HashMap<ast::Id, ConcreteInvoke<'a>>,

    /// Remaining assigmments for a given invoke.
    remaining_assigns: HashMap<ast::Id, HashSet<ast::Id>>,

    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    obligations: FactMap,

    /// Set of assumed facts
    facts: FactMap,
}

impl<'a> From<&'a HashMap<ast::Id, &'a ast::Signature>> for Context<'a> {
    fn from(sigs: &'a HashMap<ast::Id, &'a ast::Signature>) -> Self {
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
        name: ast::Id,
        comp: &ast::Id,
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
        name: ast::Id,
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
        bind: ast::Id,
        comp: &ast::Id,
    ) -> FilamentResult<()> {
        let sig = self.get_instance(comp)?;
        let ports = sig
            .inputs
            .iter()
            .chain(sig.interface_signals.iter())
            .filter_map(|pd| pd.liveness.as_ref().map(|_| &pd.name))
            .cloned()
            .collect();
        self.remaining_assigns.insert(bind, ports);
        Ok(())
    }

    pub fn remove_remaning_assign(
        &mut self,
        port: &ast::Port,
    ) -> FilamentResult<()> {
        match port {
            ast::Port::CompPort { comp, name } => {
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
            ast::Port::ThisPort(_) | ast::Port::Constant(_) => Ok(()),
        }
    }

    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F, span: Option<errors::Span>)
    where
        F: Iterator<Item = ast::Constraint>,
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
        F: Iterator<Item = ast::Constraint>,
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
        inst: &ast::Id,
    ) -> FilamentResult<&'a ast::Signature> {
        self.instances.get(inst).copied().ok_or_else(|| {
            Error::undefined(inst.clone(), "instance".to_string())
        })
    }

    /// Get the instance associated with `instance`
    pub fn get_invoke(
        &self,
        instance: &ast::Id,
    ) -> FilamentResult<&ConcreteInvoke> {
        self.invocations.get(instance).ok_or_else(|| {
            Error::undefined(instance.clone(), "invocation".to_string())
        })
    }

    /// Return the remaining assignments in this context
    pub fn get_remaining_assigns(
        &self,
    ) -> impl Iterator<Item = (&ast::Id, &HashSet<ast::Id>)> {
        self.remaining_assigns
            .iter()
            .filter(|(_, ports)| !ports.is_empty())
    }

    /// Return the guarantees of the port if any.
    /// - None if the port does not provide any guarantees.
    /// - Some(None) if the port is infinitely active (like a constant port).
    /// - Some(Some(int)) if the port is active during interval `int`.
    pub fn port_guarantees(
        &self,
        port: &ast::Port,
    ) -> FilamentResult<Option<Option<ast::Interval>>> {
        Ok(match port {
            ast::Port::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                Some(None)
            }
            ast::Port::ThisPort(port) => self
                .get_invoke(&super::THIS.into())?
                .port_guarantees(port)?
                .map(Some),
            ast::Port::CompPort { comp, name } => {
                self.get_invoke(comp)?.port_guarantees(name)?.map(Some)
            }
        })
    }

    pub fn port_requirements(
        &self,
        port: &ast::Port,
    ) -> FilamentResult<Option<ast::Interval>> {
        match port {
            ast::Port::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                unreachable!("destination port cannot be a constant")
            }
            ast::Port::ThisPort(port) => Ok(self
                .get_invoke(&super::THIS.into())?
                .port_requirements(port)?),
            ast::Port::CompPort { comp, name } => {
                Ok(self.get_invoke(comp)?.port_requirements(name)?)
            }
        }
    }
}
