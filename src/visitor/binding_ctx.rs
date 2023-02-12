//! Context that tracks the binding information in a particular program
use crate::{
    core::{self, Id, PortParam, TimeRep, WidthRep, WithTime},
    errors::{Error, FilamentResult, WithPos},
};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of a signature bound in the program
pub enum SigIdx {
    /// An external component
    Ext(usize),
    /// A Filament component
    Comp(usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an instance bound in a component
/// Defined methods represent operations on an instance and require a
/// component binding to be resolved.
pub struct InstIdx(usize);

impl InstIdx {
    /// Returns all the invocations associated with an instance
    pub fn get_all_invokes<'a, T: TimeRep, W: WidthRep>(
        &'a self,
        ctx: &'a CompBinding<'a, T, W>,
    ) -> impl Iterator<Item = InvIdx> + '_ {
        ctx.invocations
            .iter()
            .enumerate()
            .filter_map(move |(idx, inv)| {
                if inv.instance == *self {
                    Some(InvIdx(idx))
                } else {
                    None
                }
            })
    }

    /// Get the signature of this instance by resolving against the parameter bindings.
    /// Note that such a signature still has unresolved event bindings (such as the delay of a Register)
    /// that are only resolved through an invocation.
    fn param_resolved_signature<T: TimeRep, W: WidthRep>(
        &self,
        ctx: &CompBinding<'_, T, W>,
    ) -> core::Signature<T, W> {
        let inst = &ctx.instances[self.0];
        match inst.sig {
            SigIdx::Ext(idx) => {
                ctx.prog.externals[idx].resolve(&inst.params).unwrap()
            }
            SigIdx::Comp(idx) => {
                ctx.prog.components[idx].resolve(&inst.params).unwrap()
            }
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an invocation bound in a component.
/// Defined methods represent operations on an invocation and require a
/// component binding to be resolved.
pub struct InvIdx(usize);

impl InvIdx {
    /// Get resolved event bindings for the invocation
    pub fn resolved_event_binding<T: TimeRep<Offset = W>, W: WidthRep>(
        &self,
        ctx: &CompBinding<T, W>,
    ) -> Vec<T> {
        let inv = &ctx.invocations[self.0];
        let inst = &ctx.instances[inv.instance.0];
        let param_b = ctx.prog.param_binding(inst.sig, &inst.params);

        inv.events
            .iter()
            .map(|e| e.resolve_offset(&param_b))
            .collect()
    }

    /// Return the signature of the component being invoked using the parameter bindings and
    /// the event bindings of the invocation.
    pub fn resolved_signature<T: TimeRep<Offset = W>, W: WidthRep>(
        &self,
        ctx: &CompBinding<T, W>,
    ) -> core::Signature<T, W> {
        let inv = &ctx.invocations[self.0];
        let inst_idx = inv.instance;
        let inst = &ctx.instances[inst_idx.0];
        let event_b = ctx.prog.event_binding(inst.sig, &inv.events);
        inst_idx
            .param_resolved_signature(ctx)
            .resolve_event(&event_b)
    }

    /// Get the "active range" for an event in the invocation.
    /// If we have something like:
    /// ```
    /// comp Reg<G: L-(G+2), L: L-G>(...) { ... }
    ///
    /// comp main<T: 1> {
    ///   R := new Reg;
    ///   r0 := R<T+3, T+6>
    /// }
    /// ```
    ///
    /// The active ranges for events in the signature are:
    /// G -> [T+3, T+3+Delay(G)] = [T+3, T+4]
    /// L -> [T+5, T+5+Delay(L)] = [T+6, T+9]
    ///
    /// The function returns the (start_time, delay) for each event in the signature.
    pub fn event_active_ranges<T: TimeRep<Offset = W>, W: WidthRep>(
        &self,
        ctx: &CompBinding<T, W>,
    ) -> Vec<(T, T::SubRep)> {
        let inv = &ctx.invocations[self.0];
        let sig = self.resolved_signature(ctx);
        sig.events
            .iter()
            .zip(&inv.events)
            .map(|(ev, bind)| (bind.clone(), ev.delay.clone()))
            .collect_vec()
    }

    /// Fully resolve a port.
    /// Returns None if and only if the invocation is not defined
    ///
    /// Accepts a function to resolve the liveness of the port using time and width bindings.
    pub fn get_invoke_port<T: TimeRep, W: WidthRep, F>(
        &self,
        ctx: &CompBinding<T, W>,
        port: &Id,
        resolve_range: F,
    ) -> Option<core::PortDef<T, W>>
    where
        F: Fn(
            &core::Range<T>,
            &core::Binding<T>,
            &core::Binding<W>,
        ) -> core::Range<T>,
    {
        let inv = &ctx.invocations[self.0];
        let inst = &ctx.instances[inv.instance.0];
        let param_b = ctx.prog.param_binding(inst.sig, &inst.params);
        let event_b = ctx.prog.event_binding(inst.sig, &inv.events);

        match inst.sig {
            SigIdx::Ext(idx) => {
                let sig = ctx.prog.externals[idx];
                let port = sig.get_port(port);
                Some(core::PortDef::new(
                    port.name.clone(),
                    resolve_range(&port.liveness, &event_b, &param_b),
                    port.bitwidth.resolve(&param_b).unwrap(),
                ))
            }
            SigIdx::Comp(idx) => {
                let sig = &ctx.prog.components[idx];
                let port = sig.get_port(port);
                Some(core::PortDef::new(
                    port.name.clone(),
                    resolve_range(&port.liveness, &event_b, &param_b),
                    port.bitwidth.resolve(&param_b).unwrap(),
                ))
            }
        }
    }

    /// Get all the fully resolved constraints for the signature of an invocation.
    /// This includes:
    /// - The constraints of the component
    /// - Well-formedness constraints
    pub fn get_resolved_sig_constraints<T: TimeRep, W: WidthRep, F>(
        &self,
        ctx: &CompBinding<T, W>,
        resolve_constraint: F,
    ) -> Vec<core::Constraint<T>>
    where
        F: Fn(
            &core::Constraint<T>,
            &core::Binding<T>,
            &core::Binding<W>,
        ) -> core::Constraint<T>,
    {
        let inv = &ctx.invocations[self.0];
        let inst = &ctx[inv.instance];
        let sig = inst.sig;
        let param_b = &ctx.prog.param_binding(sig, &inst.params);
        let event_b = &ctx.prog.event_binding(sig, &inv.events);
        let resolve_ref = |c| resolve_constraint(c, event_b, param_b);
        let resolve = |c| resolve_constraint(&c, event_b, param_b);
        match sig {
            SigIdx::Ext(idx) => {
                let sig = ctx.prog.externals[idx];
                sig.constraints
                    .iter()
                    .map(resolve_ref)
                    .chain(sig.well_formed().map(resolve))
                    .collect()
            }
            SigIdx::Comp(idx) => {
                let sig = ctx.prog.components[idx];
                sig.constraints
                    .iter()
                    .map(resolve_ref)
                    .chain(sig.well_formed().map(resolve))
                    .collect()
            }
        }
    }
}

/// An instance bound by a component
pub struct BoundInstance<W: WidthRep> {
    /// The signature of this instance
    pub sig: SigIdx,
    /// Parameter binding for this instance
    pub params: Vec<W>,
}

pub struct BoundInvoke<T: TimeRep> {
    /// The instance being invoked
    pub instance: InstIdx,
    /// Event binding for this invocation
    pub events: Vec<T>,
}

/// Track binding information for a component
pub struct CompBinding<'p, T: TimeRep, W: WidthRep> {
    /// Context associated with the program
    pub prog: &'p ProgBinding<'p, T, W>,
    /// Signature associated with this component
    sig: SigIdx,
    /// Instances bound in this component
    instances: Vec<BoundInstance<W>>,
    /// Invocations bound in this component
    invocations: Vec<BoundInvoke<T>>,
    /// Mapping from name of instance to its index
    inst_map: HashMap<Id, InstIdx>,
    /// Mapping from name of invocation to its index
    inv_map: HashMap<Id, InvIdx>,
}

// Index into the component binding using InstIdx
impl<'p, T: TimeRep, W: WidthRep> std::ops::Index<InstIdx>
    for CompBinding<'p, T, W>
{
    type Output = BoundInstance<W>;
    fn index(&self, idx: InstIdx) -> &Self::Output {
        &self.instances[idx.0]
    }
}

// Index into the component binding using InvIdx
impl<'p, T: TimeRep, W: WidthRep> std::ops::Index<InvIdx>
    for CompBinding<'p, T, W>
{
    type Output = BoundInvoke<T>;
    fn index(&self, idx: InvIdx) -> &Self::Output {
        &self.invocations[idx.0]
    }
}

impl<'p, T: TimeRep, W: WidthRep> CompBinding<'p, T, W> {
    /// Construct a new binding context for a component
    pub fn new(
        prog_ctx: &'p ProgBinding<'p, T, W>,
        comp: &core::Component<T, W>,
    ) -> FilamentResult<Self> {
        Self::from_comp_data(prog_ctx, &comp.sig.name, &comp.body)
    }

    pub fn from_comp_data(
        prog: &'p ProgBinding<'p, T, W>,
        comp: &core::Id,
        cmds: &Vec<core::Command<T, W>>,
    ) -> FilamentResult<Self> {
        let sig = prog.find_sig_idx(comp).unwrap();
        let mut ctx = Self {
            prog,
            sig,
            instances: Vec::new(),
            invocations: Vec::new(),
            inst_map: HashMap::new(),
            inv_map: HashMap::new(),
        };

        for cmd in cmds {
            match cmd {
                core::Command::Instance(inst) => {
                    if ctx.add_instance(inst).is_none() {
                        return Err(Error::undefined(
                            inst.component.clone(),
                            "component",
                        )
                        .add_note(
                            "Component is not bound",
                            inst.component.copy_span(),
                        ));
                    }
                }
                core::Command::Invoke(inv) => {
                    if ctx.add_invoke(inv).is_none() {
                        return Err(Error::undefined(
                            inv.instance.clone(),
                            "instance",
                        )
                        .add_note(
                            "Instance is not bound",
                            inv.instance.copy_span(),
                        ));
                    }
                }
                _ => (),
            }
        }

        Ok(ctx)
    }

    /// Get the **unresolved** signature associated with this component.
    /// If this signature should be completely resolved, use [[InvIdx::resolve_signature]].
    pub fn this(&self) -> &core::Signature<T, W> {
        &self.prog.comp_sig(self.sig)
    }

    /// Return instances associated with this component
    pub fn instances(&self) -> impl Iterator<Item = InstIdx> {
        (0..self.instances.len()).map(InstIdx)
    }

    /// Return the invocations associated with this component
    pub fn invocations(&self) -> impl Iterator<Item = InvIdx> {
        (0..self.invocations.len()).map(InvIdx)
    }

    /// Signature associated with this component
    pub fn sig(&self) -> SigIdx {
        self.sig
    }

    /// Get instance binding for a given instance name
    pub fn get_instance(&self, name: &Id) -> &BoundInstance<W> {
        let idx = self.get_instance_idx(name).unwrap();
        &self[idx]
    }

    /// Get invocation binding for a given invocation name
    pub fn get_invoke(&self, name: &Id) -> &BoundInvoke<T> {
        let idx = self.get_invoke_idx(name).unwrap();
        &self[idx]
    }

    /// Get the signature associated with an invoke
    pub fn get_invoke_sig(&self, invoke: &Id) -> SigIdx {
        let inv = self.get_invoke(invoke);
        let inst = &self[inv.instance];
        inst.sig
    }

    /// Get the index for a given instance name
    pub fn get_instance_idx(&self, name: &Id) -> Option<InstIdx> {
        self.inst_map.get(name).cloned()
    }
    /// Get the index for a given invocation name
    pub fn get_invoke_idx(&self, name: &Id) -> Option<InvIdx> {
        self.inv_map.get(name).cloned()
    }

    /// Add a new instance to this binding.
    /// Returns None when the component is not bound.
    pub fn add_instance(
        &mut self,
        inst: &core::Instance<W>,
    ) -> Option<InstIdx> {
        let sig = self.prog.find_sig_idx(&inst.component)?;
        let idx = InstIdx(self.instances.len());
        self.instances.push(BoundInstance {
            sig,
            params: inst.bindings.clone(),
        });
        // Add the name to the map
        self.inst_map.insert(inst.name.clone(), idx);
        Some(idx)
    }

    /// Add a new invocation to this binding. Fully resolves the binding
    /// by filling in the default arguments.
    /// Returns `None` when the provided instance is not bound.
    pub fn add_invoke(&mut self, inv: &core::Invoke<T>) -> Option<InvIdx> {
        let instance = self.inst_map.get(&inv.instance)?;
        let binding = self
            .prog
            .event_binding(self.instances[instance.0].sig, &inv.abstract_vars);
        let idx = InvIdx(self.invocations.len());
        self.invocations.push(BoundInvoke {
            instance: *instance,
            events: binding.into_iter().map(|b| b.1).collect(),
        });
        self.inv_map.insert(inv.name.clone(), idx);
        Some(idx)
    }

    /// Returns a resolved port definition for the given port.
    /// Returns `None` if and only if the given port is a constant.
    pub fn get_resolved_port<F>(
        &self,
        port: &core::Port,
        resolve_liveness: F,
    ) -> Option<core::PortDef<T, W>>
    where
        F: Fn(
            &core::Range<T>,
            &core::Binding<T>,
            &core::Binding<W>,
        ) -> core::Range<T>,
    {
        match &port.typ {
            core::PortType::ThisPort(p) => {
                Some(self.prog.comp_sig(self.sig).get_port(p).clone())
            }
            core::PortType::InvPort { invoke, name } => Some(
                self.get_invoke_idx(invoke)
                    .unwrap()
                    .get_invoke_port(self, name, resolve_liveness)
                    .unwrap(),
            ),
            core::PortType::Constant(_) => None,
        }
    }
}

/// Signatures bound in a program.
/// Also acts a dispatcher for methods on [[core::Signature]] since external and
/// component signatures have different types.
pub struct ProgBinding<'a, T: TimeRep, W: WidthRep> {
    externals: Vec<&'a core::Signature<T, PortParam>>,
    components: Vec<&'a core::Signature<T, W>>,
    name_map: HashMap<Id, SigIdx>,
}

impl<'a, T: TimeRep, W: WidthRep> ProgBinding<'a, T, W> {
    /// Get index associated with a signature
    fn find_sig_idx(&self, name: &core::Id) -> Option<SigIdx> {
        if let Some(idx) = self.externals.iter().position(|s| *name == s.name) {
            Some(SigIdx::Ext(idx))
        } else {
            self.components
                .iter()
                .position(|s| *name == s.name)
                .map(SigIdx::Comp)
        }
    }

    /// Add a component signature to the program binding
    pub fn add_component(&mut self, sig: &'a core::Signature<T, W>) -> SigIdx {
        let idx = SigIdx::Comp(self.components.len());
        self.components.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Add a component signature to the program binding
    pub fn add_external(
        &mut self,
        sig: &'a core::Signature<T, PortParam>,
    ) -> SigIdx {
        let idx = SigIdx::Ext(self.externals.len());
        self.externals.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    // ============= Dispatch methods on Signatures =============

    /// Apply function on either an external or component signature and return the result
    #[inline]
    pub fn map_signature<O>(
        &self,
        sig: SigIdx,
        ext: impl Fn(&'a core::Signature<T, PortParam>) -> O,
        comp: impl Fn(&'a core::Signature<T, W>) -> O,
    ) -> O {
        match sig {
            SigIdx::Ext(idx) => ext(self.externals[idx]),
            SigIdx::Comp(idx) => comp(self.components[idx]),
        }
    }

    /// Get all the outputs in a signature
    pub fn output_names(&self, sig: SigIdx) -> Vec<&Id> {
        self.map_signature(
            sig,
            |ext| ext.outputs().map(|pd| &pd.name).collect_vec(),
            |comp| comp.outputs().map(|pd| &pd.name).collect_vec(),
        )
    }

    /// Get all the inputs in a signature
    pub fn input_names(&self, sig: SigIdx) -> Vec<&Id> {
        self.map_signature(
            sig,
            |ext| ext.inputs().map(|pd| &pd.name).collect_vec(),
            |comp| comp.inputs().map(|pd| &pd.name).collect_vec(),
        )
    }

    /// Name of the events associated with a signature
    pub fn event_names(&self, sig: SigIdx) -> Vec<&Id> {
        self.map_signature(
            sig,
            |ext| ext.events.iter().map(|eb| &eb.event).collect_vec(),
            |comp| comp.events.iter().map(|eb| &eb.event).collect_vec(),
        )
    }

    /// Name of the parameters associated with a signature
    pub fn param_names(&self, sig: SigIdx) -> &Vec<Id> {
        self.map_signature(sig, |ext| &ext.params, |comp| &comp.params)
    }

    /// Get the phantom events
    pub fn phantom_events(&self, sig: SigIdx) -> Vec<Id> {
        self.map_signature(
            sig,
            |ext| ext.phantom_events().collect_vec(),
            |comp| comp.phantom_events().collect_vec(),
        )
    }

    /// Returns the underlying comp signature. Panics if the signature actually points to an external.
    pub fn comp_sig(&self, sig: SigIdx) -> &'a core::Signature<T, W> {
        match sig {
            SigIdx::Ext(_) => {
                unreachable!("comp_sig called on external signature")
            }
            SigIdx::Comp(idx) => self.components[idx],
        }
    }

    /// Event binding generated from a signature
    /// XXX: Can be constructed using the binding and the event names
    fn event_binding(&self, sig: SigIdx, events: &[T]) -> core::Binding<T> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx].event_binding(events),
            SigIdx::Comp(idx) => self.components[idx].event_binding(events),
        }
    }

    /// XXX: Can be constructed using the binding and the param names
    fn param_binding(&self, sig: SigIdx, params: &[W]) -> core::Binding<W> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx].param_binding(params),
            SigIdx::Comp(idx) => self.components[idx].param_binding(params),
        }
    }
}

impl<'a, T: TimeRep, W: WidthRep> From<&'a core::Namespace<T, W>>
    for ProgBinding<'a, T, W>
{
    fn from(ns: &'a core::Namespace<T, W>) -> Self {
        let mut ctx = ProgBinding {
            externals: Vec::new(),
            components: Vec::new(),
            name_map: HashMap::new(),
        };
        ns.externs
            .iter()
            .flat_map(|(_, comps)| comps)
            .for_each(|c| {
                ctx.add_external(c);
            });
        ns.components.iter().map(|comp| &comp.sig).for_each(|c| {
            ctx.add_component(c);
        });
        ctx
    }
}
