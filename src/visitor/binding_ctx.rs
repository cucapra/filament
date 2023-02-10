//! Context that tracks the binding information in a particular program
use crate::core::{self, Id, PortParam, TimeRep, WidthRep};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of a signature bound in the program
pub enum SigIdx {
    /// An external component
    Ext(usize),
    /// A Filament componen
    Comp(usize),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an instance bound in a component
pub struct InstIdx(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an invocation bound in a component
pub struct InvIdx(usize);

/// The type of instance binding
pub enum InstBind<'a, T: TimeRep, W: WidthRep> {
    /// Signature for external components always contains parameterized ports.
    External {
        sig: &'a core::Signature<T, core::PortParam>,
    },
    /// Filament-level components are width parameteric.
    Component { sig: &'a core::Signature<T, W> },
}

impl<'a, T: TimeRep, W: WidthRep> InstBind<'a, T, W> {
    pub fn name(&self) -> Id {
        match self {
            Self::External { sig } => sig.name.clone(),
            Self::Component { sig } => sig.name.clone(),
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
    ) -> Self {
        let mut ctx = Self {
            prog: prog_ctx,
            instances: Vec::new(),
            invocations: Vec::new(),
            inst_map: HashMap::new(),
            inv_map: HashMap::new(),
        };
        for cmd in &comp.body {
            match cmd {
                core::Command::Instance(inst) => {
                    ctx.add_instance(inst);
                }
                core::Command::Invoke(inv) => {
                    ctx.add_invoke(inv);
                }
                _ => (),
            }
        }
        ctx
    }

    /// Get instance binding for a given instance name
    pub fn get_instance(&self, idx: &Id) -> &BoundInstance<W> {
        let idx = self.get_instance_idx(idx).unwrap();
        &self[idx]
    }

    /// Get invocation binding for a given invocation name
    pub fn get_invoke(&self, idx: &Id) -> &BoundInvoke<T> {
        let idx = self.get_invoke_idx(idx).unwrap();
        &self[idx]
    }

    /// Get the signature associated with an invoke
    pub fn get_invoke_sig(&self, invoke: &Id) -> SigIdx {
        let inv = self.get_invoke(invoke);
        let inst = &self[inv.instance];
        inst.sig
    }

    /// Get the index for a given instance name
    fn get_instance_idx(&self, name: &Id) -> Option<InstIdx> {
        self.inst_map.get(name).cloned()
    }
    /// Get the index for a given invocation name
    fn get_invoke_idx(&self, name: &Id) -> Option<InvIdx> {
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

    /// Add a new invocation to this binding.
    /// Returns `None` when the provided instance is not bound.
    pub fn add_invoke(&mut self, inv: &core::Invoke<T>) -> Option<InvIdx> {
        let instance = self.inst_map.get(&inv.instance)?;
        let idx = InvIdx(self.invocations.len());
        self.invocations.push(BoundInvoke {
            instance: *instance,
            events: inv.abstract_vars.clone(),
        });
        self.inv_map.insert(inv.name.clone(), idx);
        Some(idx)
    }

    /// Fully resolve a port.
    /// Returns None if and only if the invocation is not defined
    ///
    /// Accepts a function to resolve the liveness of the port using time and width bindings.
    pub fn get_resolved_port<F>(
        &self,
        inv: &Id,
        port: &Id,
        resolve_liveness: F,
    ) -> Option<core::PortDef<T, W>>
    where
        F: Fn(
            &core::Range<T>,
            &core::Binding<T>,
            &core::Binding<W>,
        ) -> core::Range<T>,
    {
        let inv_idx = self.get_invoke_idx(inv)?;
        let inv = &self.invocations[inv_idx.0];
        let inst = &self.instances[inv.instance.0];

        match inst.sig {
            SigIdx::Ext(idx) => {
                let sig = self.prog.externals[idx];
                let port = sig.get_port(port);
                let param_b = sig.param_binding(&inst.params);
                let event_b = sig.event_binding(&inv.events);
                Some(core::PortDef::new(
                    port.name.clone(),
                    resolve_liveness(&port.liveness, &event_b, &param_b),
                    port.bitwidth.resolve(&param_b).unwrap(),
                ))
            }
            SigIdx::Comp(idx) => {
                let sig = &self.prog.components[idx];
                let port = sig.get_port(port);
                let param_b = sig.param_binding(&inst.params);
                let event_b = sig.event_binding(&inv.events);
                Some(core::PortDef::new(
                    port.name.clone(),
                    resolve_liveness(&port.liveness, &event_b, &param_b),
                    port.bitwidth.resolve(&param_b).unwrap(),
                ))
            }
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

    /// Get all the outputs in a signature
    pub fn output_names(&self, sig: SigIdx) -> Vec<&Id> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx]
                .outputs()
                .map(|pd| &pd.name)
                .collect_vec(),
            SigIdx::Comp(idx) => self.components[idx]
                .outputs()
                .map(|pd| &pd.name)
                .collect_vec(),
        }
    }

    /// Get all the inputs in a signature
    pub fn input_names(&self, sig: SigIdx) -> Vec<&Id> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx]
                .inputs()
                .map(|pd| &pd.name)
                .collect_vec(),
            SigIdx::Comp(idx) => self.components[idx]
                .inputs()
                .map(|pd| &pd.name)
                .collect_vec(),
        }
    }

    /// Get the phantom events
    pub fn phantom_events(&self, sig: SigIdx) -> Vec<Id> {
        match sig {
            SigIdx::Ext(idx) => {
                self.externals[idx].phantom_events().collect_vec()
            }
            SigIdx::Comp(idx) => {
                self.components[idx].phantom_events().collect_vec()
            }
        }
    }

    /// Get the events from a signature
    pub fn events(&self, sig: SigIdx) -> &Vec<core::EventBind<T>> {
        match sig {
            SigIdx::Ext(idx) => &self.externals[idx].events,
            SigIdx::Comp(idx) => &self.components[idx].events,
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
