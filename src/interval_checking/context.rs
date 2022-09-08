use super::THIS;
use crate::core::{self, WithTime};
use crate::errors::{self, Error, FilamentResult, WithPos};
use crate::event_checker::ast;
use crate::visitor;
use std::collections::{hash_map::Entry, HashMap, HashSet};

pub enum ConcreteInvoke<'a> {
    Concrete {
        /// Bindings for abstract variables
        binding: core::Binding<'a, ast::TimeRep>,

        /// Signature
        sig: ast::Signature<u64>,
    },
    Fsm {
        /// Internal FSM
        fsm: &'a ast::Fsm,
        /// Live time of the trigger
        live_time: ast::Interval,
    },
    This {
        /// Signature
        sig: &'a ast::Signature<u64>,
    },
}

impl<'a> ConcreteInvoke<'a> {
    /// Construct an instance from a Signature and bindings for abstract variables.
    pub fn concrete(
        binding: core::Binding<'a, ast::TimeRep>,
        sig: ast::Signature<u64>,
    ) -> Self {
        Self::Concrete { binding, sig }
    }

    /// Construct an instance for "this" component.
    pub fn this_instance(sig: &'a ast::Signature<u64>) -> Self {
        Self::This { sig }
    }

    /// Construct an FSM instance
    pub fn fsm_instance(live_time: ast::Interval, fsm: &'a ast::Fsm) -> Self {
        Self::Fsm { live_time, fsm }
    }

    /// Return the width of a port
    pub fn port_width<const IS_INPUT: bool>(
        &self,
        port: &ast::Id,
    ) -> Option<u64> {
        match self {
            Self::Concrete { sig, .. } => {
                if IS_INPUT {
                    sig.inputs.iter().find_map(|p| {
                        if p.name == port {
                            Some(p.bitwidth)
                        } else {
                            None
                        }
                    })
                } else {
                    sig.outputs.iter().find_map(|p| {
                        if p.name == port {
                            Some(p.bitwidth)
                        } else {
                            None
                        }
                    })
                }
            }
            Self::This { sig } => {
                if IS_INPUT {
                    sig.inputs.iter().find_map(|p| {
                        if p.name == port {
                            Some(p.bitwidth)
                        } else {
                            None
                        }
                    })
                } else {
                    sig.outputs.iter().find_map(|p| {
                        if p.name == port {
                            Some(p.bitwidth)
                        } else {
                            None
                        }
                    })
                }
            }
            Self::Fsm { .. } => None,
        }
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
            ConcreteInvoke::Fsm { live_time, fsm } => {
                let state = core::Fsm::state(port)?;
                Ok(fsm.liveness(live_time, state))
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

    pub fn get_sig(&self) -> &ast::Signature<u64> {
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
    /// Signatures for external primitives
    sigs: &'a visitor::Bindings<'a>,

    /// Mapping for the names of active instances
    instances: HashMap<ast::Id, visitor::ResolvedInstance<'a>>,

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

impl<'a> From<&'a visitor::Bindings<'a>> for Context<'a> {
    fn from(sigs: &'a visitor::Bindings<'a>) -> Self {
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
        bindings: &[u64],
        pos: Option<errors::Span>,
    ) -> FilamentResult<()> {
        let sig = self.sigs.get(comp, bindings, pos)?;
        if self.instances.insert(name.clone(), sig).is_some() {
            return Err(Error::already_bound(name, "instance".to_string()));
        }
        Ok(())
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
            .input_names()
            .into_iter()
            .chain(sig.input_names().into_iter())
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
    pub fn add_fact(&mut self, fact: ast::Constraint) {
        log::trace!("adding known fact {}", fact);
        self.facts.push(fact);
    }

    /// Get the signature of the instance associated with `inst`
    pub fn get_instance(
        &self,
        inst: &ast::Id,
    ) -> FilamentResult<&visitor::ResolvedInstance> {
        self.instances.get(inst).ok_or_else(|| {
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
            ast::PortType::ThisPort(port) => {
                Some(self.get_invoke(&THIS.into())?.port_guarantees(port)?)
            }

            ast::PortType::CompPort { comp, name } => {
                Some(self.get_invoke(comp)?.port_guarantees(name)?)
            }
        })
    }

    // Returns the width of a port
    pub fn get_port_width<const IS_INPUT: bool>(
        &self,
        port: &ast::Port,
    ) -> FilamentResult<Option<u64>> {
        Ok(match &port.typ {
            ast::PortType::Constant(_) => None,
            ast::PortType::ThisPort(port) => {
                self.get_invoke(&THIS.into())?.port_width::<IS_INPUT>(port)
            }
            ast::PortType::CompPort { comp, name } => {
                self.get_invoke(comp)?.port_width::<IS_INPUT>(name)
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
            ast::PortType::ThisPort(port) => {
                Ok(self.get_invoke(&THIS.into())?.port_requirements(port)?)
            }
            ast::PortType::CompPort { comp, name } => {
                Ok(self.get_invoke(comp)?.port_requirements(name)?)
            }
        }
    }

    /// Construct disjointness constraints for the current context
    pub fn drain_disjointness(
        &mut self,
    ) -> FilamentResult<impl Iterator<Item = ast::Constraint>> {
        let evs = std::mem::take(&mut self.event_binds);
        let disj = evs
            .into_iter()
            .map(|(inst, binds)| self.disjointness(inst, &binds))
            .collect::<FilamentResult<Vec<_>>>()?;

        Ok(disj.into_iter().flatten())
    }

    pub fn drain_obligations(
        &mut self,
    ) -> impl Iterator<Item = ast::Constraint> {
        std::mem::take(&mut self.obligations).into_iter()
    }

    /// Generate disjointness constraints for an instance's event bindings.
    fn disjointness(
        &self,
        instance: ast::Id,
        args: &[BindsWithLoc],
    ) -> FilamentResult<Vec<ast::Constraint>> {
        // Get the delay associated with each event.
        let sig = self.get_instance(&instance)?;

        // Iterate over each event
        let mut constraints = Vec::new();
        for (idx, abs) in sig.abstract_vars().iter().enumerate() {
            // If there is no interface port associated with an event, it is ignored.
            // This only happens for primitive components.
            if let Some(id) = sig.get_interface(abs) {
                // For each event
                for (i, (spi, bi)) in args.iter().enumerate() {
                    // Delay implied by the i'th binding
                    let i_delay = id.delay().resolve(&sig.binding(bi)?);
                    // The i'th use conflicts with all other uses
                    for (k, (spk, bk)) in args.iter().enumerate() {
                        if i == k {
                            continue;
                        }

                        constraints.push(
                            ast::Constraint::from(ast::CBS::gte(
                                bi[idx].clone() - bk[idx].clone(),
                                i_delay.clone(),
                            ))
                            .add_note(
                                format!(
                                    "Conflicting invoke. Invoke provides binding {instance}.{abs}={}",
                                    bk[idx],
                                ),
                                spk.clone(),
                            )
                            .add_note(
                                format!(
                                    "Invoke provides binding {instance}.{abs}={}",
                                    bi[idx],
                                ),
                                spi.clone(),
                            )
                            .add_note(
                                format!(
                                    "@interface for {abs} specifies that invokes must be {} cycles apart",
                                    i_delay
                                ),
                                id.copy_span()
                            )
                        )
                    }
                }
            }
        }

        Ok(constraints)
    }
}
