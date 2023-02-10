//! Context that tracks the binding information in a particular program
use std::collections::HashMap;

use itertools::Itertools;

use crate::core::{self, Id, PortParam, TimeRep, WidthRep};

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
    sig: SigIdx,
    /// Parameter binding for this instance
    params: Vec<W>,
}

pub struct BoundInvoke<T: TimeRep> {
    /// The instance being invoked
    instance: InstIdx,
    /// Event binding for this invocation
    events: Vec<T>,
}

/// Track binding information for a component
pub struct CompBinding<'p, 'c, T: TimeRep, W: WidthRep> {
    /// Context associated with the program
    prog_ctx: &'p ProgBinding<'p, T, W>,
    /// This component's signature
    sig: &'c core::Signature<T, W>,
    /// Instances bound in this component
    instances: Vec<BoundInstance<W>>,
    /// Invocations bound in this component
    invocations: Vec<BoundInvoke<T>>,

    /// Mapping from name of instance to its index
    inst_map: HashMap<Id, InstIdx>,
    /// Mapping from name of invocation to its index
    inv_map: HashMap<Id, InvIdx>,
}

impl<'p, 'c, T: TimeRep, W: WidthRep> CompBinding<'p, 'c, T, W> {
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
        let sig = self.prog_ctx.find_sig_idx(&inst.component)?;
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
                let sig = self.prog_ctx.externals[idx];
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
                let sig = &self.prog_ctx.components[idx];
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
pub struct ProgBinding<'a, T: TimeRep, W: WidthRep> {
    externals: Vec<&'a core::Signature<T, PortParam>>,
    components: Vec<core::Signature<T, W>>,
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
    pub fn add_component(&mut self, sig: core::Signature<T, W>) -> SigIdx {
        let idx = SigIdx::Comp(self.components.len());
        self.components.push(sig);
        idx
    }
}

impl<'a, T: TimeRep, W: WidthRep> From<&'a core::Namespace<T, W>>
    for ProgBinding<'a, T, W>
{
    fn from(ns: &'a core::Namespace<T, W>) -> Self {
        let externals =
            ns.externs.iter().flat_map(|(_, comps)| comps).collect_vec();
        Self {
            externals,
            components: vec![],
        }
    }
}
