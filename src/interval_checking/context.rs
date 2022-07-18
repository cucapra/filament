use crate::core::{self, FsmIdxs};
use crate::errors::{self, Error, FilamentResult};
use crate::event_checker::ast;
use crate::interval_checking::solver;
use itertools::Itertools;
use std::collections::{hash_map::Entry, HashMap, HashSet};

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
    fn resolve_port<const IS_INPUT: bool>(
        &self,
        port: &ast::Id,
    ) -> FilamentResult<ast::Interval> {
        match self {
            ConcreteInvoke::Concrete { binding, sig } => {
                let live = sig.get_liveness::<IS_INPUT>(port)?;
                Ok(live.resolve(binding))
            }
            ConcreteInvoke::Fsm { start_time, fsm } => {
                let state = core::Fsm::state(port)?;
                Ok(fsm.liveness(start_time, state))
            }
            ConcreteInvoke::This { sig } => {
                Ok(sig.get_liveness::<IS_INPUT>(port)?)
            }
        }
    }

    /// Returns the requirements of an input port.
    pub fn port_requirements(
        &self,
        port: &ast::Id,
    ) -> FilamentResult<ast::Interval> {
        self.resolve_port::<true>(port)
    }

    /// Returns the guarantees provided by an output port
    pub fn port_guarantees(
        &self,
        port: &ast::Id,
    ) -> FilamentResult<ast::Interval> {
        self.resolve_port::<false>(port)
    }

    pub fn get_sig(&self) -> &ast::Signature {
        match &self {
            ConcreteInvoke::Concrete { sig, .. } => sig,
            ConcreteInvoke::This { sig } => sig,
            ConcreteInvoke::Fsm { .. } => {
                unreachable!("Called get_sig on FSM instance")
            }
        }
    }
}

type FactMap = Vec<ast::Constraint>;
type BindsWithLoc = (Option<errors::Span>, Vec<ast::TimeRep>);

pub struct Context<'a> {
    /// Mapping from names to signatures for components and externals.
    sigs: &'a HashMap<ast::Id, &'a ast::Signature>,

    /// Mapping for the names of active instances
    instances: HashMap<ast::Id, &'a ast::Signature>,

    /// Mapping from name of invocations to their information
    invocations: HashMap<ast::Id, ConcreteInvoke<'a>>,

    /// Remaining assigmments for a given invoke.
    remaining_assigns: HashMap<ast::Id, HashSet<ast::Id>>,

    /// Mapping from instance to event bindings
    event_binds: HashMap<ast::Id, Vec<BindsWithLoc>>,

    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    obligations: FactMap,

    /// Set of assumed facts
    pub facts: FactMap,
}

impl<'a> From<&'a HashMap<ast::Id, &'a ast::Signature>> for Context<'a> {
    fn from(sigs: &'a HashMap<ast::Id, &'a ast::Signature>) -> Self {
        Context {
            sigs,
            remaining_assigns: HashMap::default(),
            instances: HashMap::default(),
            invocations: HashMap::default(),
            event_binds: HashMap::default(),
            obligations: Vec::default(),
            facts: Vec::default(),
        }
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

    /// Add assignments that must be present to make a low-level invoke work
    /// correctly.
    pub fn add_remaning_assigns(
        &mut self,
        bind: ast::Id,
        comp: &ast::Id,
    ) -> FilamentResult<()> {
        let sig = self.get_instance(comp)?;
        let ports = sig
            .inputs
            .iter()
            .map(|pd| &pd.name)
            .chain(sig.interface_signals.iter().map(|id| &id.name))
            .cloned()
            .collect();
        self.remaining_assigns.insert(bind, ports);
        Ok(())
    }

    /// Track event bindings for each instance.
    /// This is used for the disjointness check.
    pub fn add_event_binds(
        &mut self,
        instance: ast::Id,
        binds: Vec<ast::TimeRep>,
        pos: Option<errors::Span>,
    ) {
        self.event_binds
            .entry(instance)
            .or_default()
            .push((pos, binds));
    }

    /// Remove a remaining assignment from an invoke
    pub fn remove_remaning_assign(
        &mut self,
        port: &ast::Port,
    ) -> FilamentResult<()> {
        match &port.typ {
            ast::PortType::CompPort { comp, name } => {
                // Check if the port is defined
                self.get_invoke(comp)?.resolve_port::<true>(name)?;
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
            ast::PortType::ThisPort(_) | ast::PortType::Constant(_) => Ok(()),
        }
    }

    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F)
    where
        F: Iterator<Item = ast::Constraint>,
    {
        for fact in facts {
            log::trace!("adding obligation {}", fact);
            self.obligations.push(fact);
        }
    }

    /// Add a new known fact
    pub fn add_fact<F>(&mut self, facts: F)
    where
        F: Iterator<Item = ast::Constraint>,
    {
        for fact in facts {
            log::trace!("adding known fact {}", fact);
            self.facts.push(fact);
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
    /// - None if the port is infinitely active (like a constant port).
    pub fn port_guarantees(
        &self,
        port: &ast::Port,
    ) -> FilamentResult<Option<ast::Interval>> {
        Ok(match &port.typ {
            ast::PortType::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                None
            }
            ast::PortType::ThisPort(port) => Some(
                self.get_invoke(&super::THIS.into())?
                    .port_guarantees(port)?,
            ),

            ast::PortType::CompPort { comp, name } => {
                Some(self.get_invoke(comp)?.port_guarantees(name)?)
            }
        })
    }

    pub fn port_requirements(
        &self,
        port: &ast::Port,
    ) -> FilamentResult<ast::Interval> {
        match &port.typ {
            ast::PortType::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                unreachable!("destination port cannot be a constant")
            }
            ast::PortType::ThisPort(port) => Ok(self
                .get_invoke(&super::THIS.into())?
                .port_requirements(port)?),
            ast::PortType::CompPort { comp, name } => {
                Ok(self.get_invoke(comp)?.port_requirements(name)?)
            }
        }
    }

    /// Construct disjointness constraints for the current context
    pub fn drain_disjointness(
        &mut self,
    ) -> FilamentResult<impl Iterator<Item = solver::Disjoint>> {
        let evs = std::mem::take(&mut self.event_binds);
        let disj = evs
            .into_iter()
            .map(|(inst, binds)| self.disjointness(inst, binds))
            .collect::<FilamentResult<Vec<_>>>()?;

        Ok(disj.into_iter().flatten())
    }

    pub fn drain_obligations(&mut self) -> Vec<ast::Constraint> {
        std::mem::take(&mut self.obligations)
    }

    /// Generate disjointness constraints for an instance's event bindinds.
    fn disjointness(
        &self,
        instance: ast::Id,
        binds: Vec<BindsWithLoc>,
    ) -> FilamentResult<Vec<solver::Disjoint>> {
        // Get the delay associated with each event.
        let sig = self.get_instance(&instance)?;

        // If there is no interface port associated with an event, it is ignored.
        // This only happens for primitive components.
        let delays = sig
            .abstract_vars
            .iter()
            .map(|ev| {
                sig.interface_signals
                    .iter()
                    .find(|id| id.event == ev)
                    .map(|id| id.delay().concrete().unwrap())
            })
            .collect_vec();

        // Iterate over each event
        let mut constraints = Vec::new();
        let events = delays.len();
        let num_binds = binds.len();
        (0..events).for_each(|ev| {
            if let Some(delay) = delays[ev] {
                // For each event
                for i in 0..num_binds {
                    // The i'th use conflicts with all other uses
                    for k in i + 1..num_binds {
                        let (spi, bi) = &binds[i];
                        let (spk, bk) = &binds[k];
                        constraints.push(
                            solver::Disjoint::new(
                                bi[ev].clone(),
                                bk[ev].clone(),
                                delay,
                            )
                            .add_locations(
                                spi.iter().chain(spk.iter()).cloned(),
                            ),
                        )
                    }
                }
            }
        });
        Ok(constraints)
    }
}
