use super::{ShareConstraints, THIS};
use crate::core::{self, WithTime};
use crate::errors::{self, Error, FilamentResult, WithPos};
use crate::event_checker::ast;
use crate::visitor;
use std::collections::{HashMap, HashSet};

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
    ) {
        let sig = self.sigs.get_component(comp, bindings);
        self.instances.insert(name.clone(), sig);
    }

    /// Add a new invocation to the context
    pub fn add_invocation(
        &mut self,
        name: ast::Id,
        instance: ConcreteInvoke<'a>,
    ) {
        self.invocations.insert(name, instance);
    }

    /// Add assignments that must be present to make a low-level invoke work
    /// correctly.
    pub fn add_remaning_assigns(
        &mut self,
        invoke: ast::Id,
        comp: &ast::Id,
    ) -> FilamentResult<()> {
        let sig = self.get_instance(comp);
        let ports = sig
            .input_names()
            .into_iter()
            .chain(sig.input_names().into_iter())
            .collect();
        self.remaining_assigns.insert(invoke, ports);
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
            ast::PortType::InvPort { invoke: comp, name } => {
                // Check if the port is defined
                self.get_invoke(comp).resolve_port::<true>(name)?;
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
    pub fn get_instance(&self, inst: &ast::Id) -> &visitor::ResolvedInstance {
        &self.instances[inst]
    }

    /// Get the instance associated with `instance`
    pub fn get_invoke(&self, instance: &ast::Id) -> &ConcreteInvoke {
        &self.invocations[instance]
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
                Some(self.get_invoke(&THIS.into()).port_guarantees(port)?)
            }

            ast::PortType::InvPort { invoke: comp, name } => {
                Some(self.get_invoke(comp).port_guarantees(name)?)
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
                Ok(self.get_invoke(&THIS.into()).port_requirements(port)?)
            }
            ast::PortType::InvPort { invoke: comp, name } => {
                Ok(self.get_invoke(comp).port_requirements(name)?)
            }
        }
    }

    /// Construct disjointness constraints for the current context
    pub fn drain_sharing(
        &mut self,
    ) -> FilamentResult<(Vec<ast::Constraint>, Vec<ShareConstraints>)> {
        let evs = std::mem::take(&mut self.event_binds);
        let all = evs
            .into_iter()
            .map(|(inst, binds)| self.sharing_constraints(inst, &binds))
            .collect::<FilamentResult<Vec<_>>>()?;
        let mut cons = Vec::new();
        let mut share = Vec::new();
        for (c, s) in all {
            cons.extend(c);
            share.extend(s);
        }
        Ok((cons, share))
    }

    pub fn drain_obligations(&mut self) -> Vec<ast::Constraint> {
        std::mem::take(&mut self.obligations)
    }

    fn ensure_same_events(
        &self,
        instance: &ast::Id,
        args: &[BindsWithLoc],
    ) -> FilamentResult<()> {
        // Get the delay associated with each event.
        let sig = self.get_instance(&instance);
        // Ensure that all bindings of an event variable use the same events
        for (idx, abs) in sig.abstract_vars().iter().enumerate() {
            // Ignore events without an associated interface port.
            if sig.get_interface(abs).is_some() {
                let mut iter =
                    args.iter().map(|(pos, binds)| (pos, &binds[idx]));

                if let Some((fpos, first)) = iter.next() {
                    for (epos, event) in iter {
                        if event
                            .events()
                            .map(|(ev, _)| ev)
                            .collect::<HashSet<_>>()
                            != first
                                .events()
                                .map(|(ev, _)| ev)
                                .collect::<HashSet<_>>()
                        {
                            return Err(Error::malformed(format!(
                                "Bindings for {instance}.{abs} uses multiple events: {first} and {event}. Sharing using multiple events is not supported.",
                            ))
                            .add_note(format!("Location provides binding {instance}.{abs}={first}"), fpos.clone())
                            .add_note(format!("Location provides binding {instance}.{abs}={event}"), epos.clone()));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Constraint generated for disjointness
    fn sharing_contraints(
        instance: &ast::Id,
        abs: &ast::Id,
        (i_event, spi): (&ast::TimeRep, Option<errors::Span>),
        (k_event, spk): (&ast::TimeRep, Option<errors::Span>),
        i_delay: &core::TimeSub<core::FsmIdxs>,
        id_pos: Option<errors::Span>,
    ) -> core::Constraint<core::FsmIdxs> {
        ast::Constraint::from(ast::CBS::gte(
            i_event.clone() - k_event.clone(),
            i_delay.clone(),
        ))
        .add_note(
            format!("Conflicting invoke. Invoke provides binding {instance}.{abs}={k_event}"),
            spk.clone(),
        )
        .add_note(
            format!("Invoke provides binding {instance}.{abs}={i_event}"),
            spi.clone(),
        )
        .add_note(
            format!("@interface for {abs} specifies that invokes must be {i_delay} cycles apart"),
            id_pos,
        )
    }

    /// Generate disjointness constraints for an instance's event bindings.
    fn sharing_constraints(
        &self,
        instance: ast::Id,
        args: &[BindsWithLoc],
    ) -> FilamentResult<(Vec<ast::Constraint>, Vec<ShareConstraints>)> {
        // Ensure that bindings for events variables use the same variables.
        self.ensure_same_events(&instance, args)?;

        // Get the delay associated with each event.
        let sig = self.get_instance(&instance);

        // Iterate over each event
        let mut constraints = Vec::new();
        let mut share_constraints = Vec::new();
        for (idx, abs) in sig.abstract_vars().iter().enumerate() {
            // If there is no interface port associated with an event, it is ignored.
            // This only happens for primitive components such as the Register which does
            // not define an interface port for its end time.
            if let Some(id) = sig.get_interface(abs) {
                // Track minimum and maximum end times for each binding
                let mut share = ShareConstraints::default();

                let delay = id.delay();
                // For each binding
                for (i, (spi, bi)) in args.iter().enumerate() {
                    // Delay implied by the i'th binding
                    let i_delay = delay.resolve(&sig.binding(bi));
                    // The i'th use conflicts with all other uses
                    for (k, (spk, bk)) in args.iter().enumerate() {
                        if i == k {
                            continue;
                        }

                        constraints.push(Context::sharing_contraints(
                            &instance,
                            abs,
                            (&bi[idx], spi.clone()),
                            (&bk[idx], spk.clone()),
                            &i_delay,
                            id.copy_span(),
                        ))
                    }
                    share.add_bind_info(
                        bi[idx].clone(),
                        (bi[idx].clone(), i_delay),
                        spi.clone(),
                    );
                }

                // # Constraints generated from sharing instances
                // If a instance is shared at events Gi, then for all events T defined by
                // the instance, where dT defines the delay for T, and dG is the delay for
                // G, we have:
                // dG >= max(Gi+dT.resolve(Gi)) - min(Gi)
                //
                // In other words, the delay of the events trigger the shared instance
                // should be greater that the range occupied by the invocations of the
                // instance.
                if let ConcreteInvoke::This { sig } =
                    self.get_invoke(&THIS.into())
                {
                    // Get delays for events used in bindings. These are guaranteed to be the same across all bindings
                    // due to the call to `ensure_same_events`.
                    let bind = &args[0].1[idx];
                    let delays = bind
                        .events()
                        .filter_map(|(ev, _)| sig.get_interface(ev))
                        .cloned();
                    share.add_delays(delays);
                    share_constraints.push(share);
                } else {
                    unreachable!("Signature associate with THIS is not a ConcreteInvoke::This")
                }
            }
        }

        Ok((constraints, share_constraints))
    }
}
