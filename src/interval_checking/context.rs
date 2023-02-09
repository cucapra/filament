use super::{ShareConstraints, THIS};
use crate::core::{self, Constraint, TimeRep, WidthRep, WithTime};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use crate::visitor;
use std::collections::{HashMap, HashSet};

pub enum ConcreteInvoke<'a, T: TimeRep, W: WidthRep> {
    Concrete {
        /// Bindings for abstract variables
        binding: core::Binding<T>,
        /// Signature
        sig: core::Signature<T, W>,
    },
    This {
        /// Signature
        sig: &'a core::Signature<T, W>,
    },
}

impl<'a, T: TimeRep, W: WidthRep> ConcreteInvoke<'a, T, W> {
    /// Construct an instance from a Signature and bindings for abstract variables.
    pub fn concrete(
        binding: core::Binding<T>,
        sig: core::Signature<T, W>,
    ) -> Self {
        Self::Concrete { binding, sig }
    }

    /// Construct an instance for "this" component.
    pub fn this_instance(sig: &'a core::Signature<T, W>) -> Self {
        Self::This { sig }
    }

    /// Resolve a port for this instance and return the requirement or guarantee
    /// based on whether it is an input or an input port.
    #[inline]
    fn resolve_port<const IS_INPUT: bool>(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Range<T>> {
        match self {
            ConcreteInvoke::Concrete { binding, sig } => {
                let live = sig.get_liveness::<IS_INPUT>(port)?;
                Ok(live.resolve(binding))
            }
            ConcreteInvoke::This { sig } => {
                Ok(sig.get_liveness::<IS_INPUT>(port)?)
            }
        }
    }

    /// Returns the requirements of an input port.
    pub fn port_requirements(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Range<T>> {
        self.resolve_port::<true>(port)
    }

    /// Returns the guarantees provided by an output port
    pub fn port_guarantees(
        &self,
        port: &core::Id,
    ) -> FilamentResult<core::Range<T>> {
        self.resolve_port::<false>(port)
    }

    pub fn get_event(&self, event: &core::Id) -> &core::EventBind<T> {
        match &self {
            ConcreteInvoke::Concrete { sig, .. } => sig.get_event(event),
            ConcreteInvoke::This { sig } => sig.get_event(event),
        }
    }
}

type FactMap<T> = Vec<core::Constraint<T>>;
type BindsWithLoc<T> = (GPosIdx, Vec<T>);

pub struct Context<'a, T: TimeRep, W: WidthRep> {
    /// Signatures for external primitives
    sigs: &'a visitor::Bindings<'a, T, W>,

    /// Mapping for the names of active instances
    instances: HashMap<core::Id, visitor::ResolvedInstance<'a, T, W>>,

    /// Mapping from name of invocations to their information
    invocations: HashMap<core::Id, ConcreteInvoke<'a, T, W>>,

    /// Remaining assigmments for a given invoke.
    remaining_assigns: HashMap<core::Id, HashSet<core::Id>>,

    /// Mapping from instance to event bindings
    event_binds: HashMap<core::Id, Vec<BindsWithLoc<T>>>,

    /// Set of facts that need to be proven.
    /// Mapping from facts to the locations that generated it.
    obligations: FactMap<T>,

    /// Set of assumed facts
    pub facts: FactMap<T>,
}

impl<'a, T: TimeRep, W: WidthRep> From<&'a visitor::Bindings<'a, T, W>>
    for Context<'a, T, W>
{
    fn from(sigs: &'a visitor::Bindings<'a, T, W>) -> Self {
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

impl<'a, T: TimeRep, W: WidthRep> Context<'a, T, W> {
    /// Add a new instance to the context with the signatuer from `comp`
    pub fn add_instance(
        &mut self,
        name: core::Id,
        comp: &core::Id,
        bindings: &[W],
    ) {
        let sig = self.sigs.get_component(comp, bindings);
        self.instances.insert(name, sig);
    }

    /// Add a new invocation to the context
    pub fn add_invocation(
        &mut self,
        name: core::Id,
        instance: ConcreteInvoke<'a, T, W>,
    ) {
        self.invocations.insert(name, instance);
    }

    /// Add assignments that must be present to make a low-level invoke work
    /// correctly.
    pub fn add_remaning_assigns(
        &mut self,
        invoke: core::Id,
        comp: &core::Id,
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
        instance: core::Id,
        binds: &core::Binding<T>,
        pos: GPosIdx,
    ) {
        self.event_binds
            .entry(instance)
            .or_default()
            .push((pos, binds.iter().map(|(_, t)| t.clone()).collect()));
    }

    /// Remove a remaining assignment from an invoke
    pub fn remove_remaning_assign(
        &mut self,
        port: &core::Port,
    ) -> FilamentResult<()> {
        match &port.typ {
            core::PortType::InvPort { invoke: comp, name } => {
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
            core::PortType::ThisPort(_) | core::PortType::Constant(_) => Ok(()),
        }
    }

    /// Add a new obligation that needs to be proved
    pub fn add_obligations<F>(&mut self, facts: F)
    where
        F: Iterator<Item = core::Constraint<T>>,
    {
        for fact in facts {
            log::trace!("adding obligation {}", fact);
            self.obligations.push(fact);
        }
    }

    /// Add a new known fact
    pub fn add_fact(&mut self, fact: core::Constraint<T>) {
        log::trace!("adding known fact {}", fact);
        self.facts.push(fact);
    }

    /// Get the signature of the instance associated with `inst`
    pub fn get_instance(
        &self,
        inst: &core::Id,
    ) -> &visitor::ResolvedInstance<T, W> {
        &self.instances[inst]
    }

    /// Get the instance associated with `instance`
    pub fn get_invoke(&self, instance: &core::Id) -> &ConcreteInvoke<T, W> {
        &self.invocations[instance]
    }

    /// Return the remaining assignments in this context
    pub fn get_remaining_assigns(
        &self,
    ) -> impl Iterator<Item = (&core::Id, &HashSet<core::Id>)> {
        self.remaining_assigns
            .iter()
            .filter(|(_, ports)| !ports.is_empty())
    }

    /// Return the guarantees of the port if any.
    /// - None if the port does not provide any guarantees.
    /// - None if the port is infinitely active (like a constant port).
    pub fn port_guarantees(
        &self,
        port: &core::Port,
    ) -> FilamentResult<Option<core::Range<T>>> {
        Ok(match &port.typ {
            core::PortType::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                None
            }
            core::PortType::ThisPort(port) => {
                Some(self.get_invoke(&THIS.into()).port_guarantees(port)?)
            }

            core::PortType::InvPort { invoke: comp, name } => {
                Some(self.get_invoke(comp).port_guarantees(name)?)
            }
        })
    }

    pub fn port_requirements(
        &self,
        port: &core::Port,
    ) -> FilamentResult<core::Range<T>> {
        match &port.typ {
            core::PortType::Constant(_) => {
                /* Constants do not generate a proof obligation because they are
                 * always available. */
                unreachable!("destination port cannot be a constant")
            }
            core::PortType::ThisPort(port) => {
                Ok(self.get_invoke(&THIS.into()).port_requirements(port)?)
            }
            core::PortType::InvPort { invoke: comp, name } => {
                Ok(self.get_invoke(comp).port_requirements(name)?)
            }
        }
    }

    /// Construct disjointness constraints for the current context
    pub fn drain_sharing(
        &mut self,
    ) -> FilamentResult<(Vec<Constraint<T>>, Vec<ShareConstraints<T>>)> {
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

    pub fn drain_obligations(&mut self) -> Vec<core::Constraint<T>> {
        std::mem::take(&mut self.obligations)
    }

    fn ensure_same_events(
        &self,
        instance: &core::Id,
        args: &[BindsWithLoc<T>],
    ) -> FilamentResult<()> {
        // Get the delay associated with each event.
        let sig = self.get_instance(instance);
        // Ensure that all bindings of an event variable use the same events
        for (idx, abs) in sig.events().iter().enumerate() {
            // Ignore events without an associated interface port.
            if sig.get_interface(abs).is_some() {
                let mut iter =
                    args.iter().map(|(pos, binds)| (pos, &binds[idx]));

                if let Some((fpos, first)) = iter.next() {
                    for (epos, event) in iter {
                        if event.event() != first.event() {
                            return Err(Error::malformed(format!(
                                "Bindings for {instance}.{abs} uses multiple events: {first} and {event}. Sharing using multiple events is not supported.",
                            ))
                            .add_note(format!("Location provides binding {instance}.{abs}={first}"), *fpos)
                            .add_note(format!("Location provides binding {instance}.{abs}={event}"), *epos));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Constraint generated for disjointness
    fn sharing_contraints(
        instance: &core::Id,
        abs: &core::Id,
        (i_event, spi): (&T, GPosIdx),
        (k_event, spk): (&T, GPosIdx),
        i_delay: &T::SubRep,
        id_pos: GPosIdx,
    ) -> core::Constraint<T> {
        core::Constraint::sub(core::OrderConstraint::gte(
            i_event.clone().sub(k_event.clone()),
            i_delay.clone(),
        ))
        .add_note(
            format!("Conflicting invoke. Invoke provides binding {instance}.{abs}={k_event}"),
            spk,
        )
        .add_note(
            format!("Invoke provides binding {instance}.{abs}={i_event}"),
            spi,
        )
        .add_note(
            format!("Delay for {abs} specifies that invokes must be {i_delay} cycles apart"),
            id_pos,
        )
    }

    /// Generate disjointness constraints for an instance's event bindings.
    fn sharing_constraints(
        &self,
        instance: core::Id,
        args: &[BindsWithLoc<T>],
    ) -> FilamentResult<(Vec<Constraint<T>>, Vec<ShareConstraints<T>>)> {
        // Ensure that bindings for events variables use the same variables.
        self.ensure_same_events(&instance, args)?;

        // Get the delay associated with each event.
        let sig = self.get_instance(&instance);

        // Iterate over each event
        let mut constraints = Vec::new();
        let mut share_constraints = Vec::new();
        for (idx, abs) in sig.events().iter().enumerate() {
            // If there is no interface port associated with an event, it is ignored.
            // This only happens for primitive components such as the Register which does
            // not define an interface port for its end time.
            if let Some(eb) = sig.get_event(abs) {
                // Track minimum and maximum end times for each binding
                let mut share = ShareConstraints::default();

                let delay = &eb.delay;
                // For each binding
                for (i, (spi, bi)) in args.iter().enumerate() {
                    // Delay implied by the i'th binding
                    let i_delay = delay.resolve(&sig.binding(bi));
                    // The i'th use conflicts with all other uses
                    for (k, (spk, bk)) in args.iter().enumerate() {
                        if i == k {
                            continue;
                        }

                        constraints.push(Self::sharing_contraints(
                            &instance,
                            abs,
                            (&bi[idx], *spi),
                            (&bk[idx], *spk),
                            &i_delay,
                            eb.copy_span(),
                        ))
                    }
                    share.add_bind_info(
                        bi[idx].clone(),
                        (bi[idx].clone(), i_delay),
                        *spi,
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
                    let ev = &bind.event();
                    let eb = sig.get_event(ev);
                    share.add_delays(std::iter::once(eb.clone()));
                    share_constraints.push(share);
                } else {
                    unreachable!("Signature associate with THIS is not a ConcreteInvoke::This")
                }
            }
        }

        Ok((constraints, share_constraints))
    }
}
