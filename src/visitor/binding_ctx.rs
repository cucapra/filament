//! Context that tracks the binding information in a particular program
use crate::{
    core::{self, Id, Time, TimeSub},
    diagnostics,
    errors::{Error, WithPos},
    utils::{self, GPosIdx},
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

impl SigIdx {
    /// The Unknown signature
    pub const UNKNOWN: SigIdx = SigIdx::Ext(usize::MAX);
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// Index of an instance bound in a component
/// Defined methods represent operations on an instance and require a
/// component binding to be resolved.
pub struct InstIdx(usize);

impl InstIdx {
    /// The Unknown instance
    pub const UNKNOWN: InstIdx = InstIdx(usize::MAX);
}

impl InstIdx {
    /// Get the position of the instance
    pub fn pos(&self, ctx: &CompBinding) -> GPosIdx {
        ctx.instances[self.0].pos
    }

    /// Returns all the invocations associated with an instance
    pub fn get_all_invokes<'a>(
        &'a self,
        ctx: &'a CompBinding<'a>,
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
    fn param_resolved_signature(&self, ctx: &CompBinding) -> core::Signature {
        let inst = &ctx.instances[self.0];
        match inst.sig {
            SigIdx::Ext(idx) => {
                ctx.prog.externals[idx].resolve_offset(&inst.params)
            }
            SigIdx::Comp(idx) => {
                ctx.prog.components[idx].resolve_offset(&inst.params)
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
    /// The Unknown invocation
    pub const UNKNOWN: InvIdx = InvIdx(usize::MAX);
}

impl InvIdx {
    /// Get the position of the invocation
    pub fn pos(&self, ctx: &CompBinding) -> GPosIdx {
        ctx.invocations[self.0].pos
    }

    /// Get resolved event bindings for the invocation
    pub fn resolved_event_binding(&self, ctx: &CompBinding) -> Vec<Time> {
        let inv = &ctx.invocations[self.0];
        let inst = &ctx.instances[inv.instance.0];
        let param_b = ctx.prog.param_binding(inst.sig, &inst.params);

        inv.events
            .iter()
            .map(|e| e.resolve_expr(&param_b))
            .collect()
    }

    /// Return the idx for the signature associated with the invocation.
    pub fn unresolved_signature(&self, ctx: &CompBinding) -> SigIdx {
        let inv = &ctx.invocations[self.0];
        let inst = &ctx[inv.instance];
        inst.sig
    }

    /// Return the signature of the component being invoked using the parameter bindings and
    /// the event bindings of the invocation.
    pub fn resolved_signature(&self, ctx: &CompBinding) -> core::Signature {
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
    pub fn event_active_ranges(
        &self,
        ctx: &CompBinding,
    ) -> Vec<(Time, TimeSub)> {
        let inv = &ctx.invocations[self.0];
        let sig = self.resolved_signature(ctx);
        sig.events
            .iter()
            .zip(&inv.events)
            .map(|(ev, bind)| (bind.clone(), ev.delay.clone()))
            .collect_vec()
    }

    /// Fully resolve a port.
    /// Accepts a function to resolve the liveness of the port using time and width bindings.
    // XXX: Does not need to return an option
    pub fn get_invoke_port<F>(
        &self,
        ctx: &CompBinding,
        port: &Id,
        resolve_range: F,
    ) -> core::PortDef
    where
        F: Fn(
            &core::Range,
            &utils::Binding<Time>,
            &utils::Binding<core::Expr>,
        ) -> core::Range,
    {
        let inv = &ctx.invocations[self.0];
        let inst = &ctx.instances[inv.instance.0];
        let param_b = ctx.prog.param_binding(inst.sig, &inst.params);
        let event_b = ctx.prog.event_binding(inst.sig, &inv.events);
        let sig = ctx.prog.sig(inst.sig);
        let port = sig.get_port(port);
        core::PortDef::new(
            port.name.clone(),
            resolve_range(&port.liveness, &event_b, &param_b),
            port.bitwidth.resolve(&param_b),
        )
    }

    /// Get all the fully resolved constraints for the signature of an invocation.
    /// This includes:
    /// - The constraints of the component
    /// - Well-formedness constraints
    pub fn get_resolved_sig_constraints<F>(
        &self,
        ctx: &CompBinding,
        resolve_constraint: F,
        diag: &mut diagnostics::Diagnostics,
    ) -> Vec<core::Constraint>
    where
        F: Fn(
            &core::Constraint,
            &utils::Binding<Time>,
            &utils::Binding<core::Expr>,
        ) -> core::Constraint,
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
                    .chain(sig.well_formed(diag).into_iter().map(resolve))
                    .collect()
            }
            SigIdx::Comp(idx) => {
                let sig = ctx.prog.components[idx];
                sig.constraints
                    .iter()
                    .map(resolve_ref)
                    .chain(sig.well_formed(diag).into_iter().map(resolve))
                    .collect()
            }
        }
    }
}

/// An instance bound by a component
pub struct BoundInstance {
    /// The signature of this instance
    pub sig: SigIdx,
    /// Parameter binding for this instance
    pub params: Vec<core::Expr>,
    /// Position associated with this instance
    pos: GPosIdx,
}

impl WithPos for BoundInstance {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

pub struct BoundInvoke {
    /// The instance being invoked
    pub instance: InstIdx,
    /// Event binding for this invocation
    pub events: Vec<Time>,
    /// Position associated with this invocation
    pos: GPosIdx,
}

impl WithPos for BoundInvoke {
    fn set_span(mut self, sp: GPosIdx) -> Self {
        self.pos = sp;
        self
    }

    fn copy_span(&self) -> GPosIdx {
        self.pos
    }
}

/// Track binding information for a component
pub struct CompBinding<'p> {
    /// Context associated with the program
    pub prog: &'p ProgBinding<'p>,
    /// Signature associated with this component
    sig: SigIdx,
    /// Instances bound in this component
    instances: Vec<BoundInstance>,
    /// Invocations bound in this component
    invocations: Vec<BoundInvoke>,
    /// Mapping from name of instance to its index
    inst_map: HashMap<Id, InstIdx>,
    /// Mapping from name of invocation to its index
    inv_map: HashMap<Id, InvIdx>,
}

// Index into the component binding using InstIdx
impl<'p> std::ops::Index<InstIdx> for CompBinding<'p> {
    type Output = BoundInstance;
    fn index(&self, idx: InstIdx) -> &Self::Output {
        &self.instances[idx.0]
    }
}

// Index into the component binding using InvIdx
impl<'p> std::ops::Index<InvIdx> for CompBinding<'p> {
    type Output = BoundInvoke;
    fn index(&self, idx: InvIdx) -> &Self::Output {
        &self.invocations[idx.0]
    }
}

impl<'p> CompBinding<'p> {
    /// Construct a new binding context for a component
    pub fn new(prog_ctx: &'p ProgBinding<'p>, comp: &core::Component) -> Self {
        Self::from_component(prog_ctx, &comp.sig.name, &comp.body)
    }

    pub fn new_checked(
        prog_ctx: &'p ProgBinding<'p>,
        comp: &core::Component,
        diag: &mut diagnostics::Diagnostics,
    ) -> Option<Self> {
        Self::from_component_checked(prog_ctx, &comp.sig.name, &comp.body, diag)
    }

    /// Construct a new instance using information from a [[core::Component]].
    pub fn from_component(
        prog: &'p ProgBinding<'p>,
        comp: &core::Id,
        cmds: &Vec<core::Command>,
    ) -> Self {
        let sig = prog.get_sig_idx(comp);
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

    /// Similar to [[Self::from_component]], does not assume that bindings are valid.
    /// Adds error information to a [[Diagnostic]] object if there are any errors.
    pub fn from_component_checked(
        prog: &'p ProgBinding<'p>,
        comp: &core::Id,
        cmds: &Vec<core::Command>,
        diag: &mut diagnostics::Diagnostics,
    ) -> Option<Self> {
        let sig = prog.get_sig_idx(comp); // Cannot throw error
        let mut ctx = Self {
            prog,
            sig,
            instances: Vec::new(),
            invocations: Vec::new(),
            inst_map: HashMap::new(),
            inv_map: HashMap::new(),
        };
        let mut has_errors = false;
        for cmd in cmds {
            match cmd {
                core::Command::Instance(inst) => {
                    let comp = &inst.component;
                    if ctx.prog.find_sig_idx(comp).is_some() {
                        ctx.add_instance(inst);
                    } else {
                        has_errors = true;
                        // If there is no component with this name, add an error and use a dummy signature
                        let err = Error::undefined(comp.clone(), "component")
                            .add_note(diag.add_info(
                                "unknown component",
                                comp.copy_span(),
                            ));
                        diag.add_error(err);
                        ctx.add_bound_instance(
                            inst.name.clone(),
                            SigIdx::UNKNOWN,
                            vec![],
                            inst.copy_span(),
                        );
                    }
                }
                core::Command::Invoke(inv) => {
                    if ctx.inst_map.get(&inv.instance).is_some() {
                        // If there have been previous errors, we cannot rely on signatures being valid
                        if has_errors {
                            ctx.add_bound_invoke(
                                inv.name.clone(),
                                InstIdx::UNKNOWN,
                                vec![],
                                inv.copy_span(),
                            );
                        } else {
                            ctx.add_invoke(inv);
                        }
                    } else {
                        has_errors = true;
                        // If there is no component with this name, add an error and use a dummy signature
                        let err =
                            Error::undefined(inv.instance.clone(), "instance")
                                .add_note(diag.add_info(
                                    "unknown instance",
                                    inv.instance.copy_span(),
                                ));
                        diag.add_error(err);
                        ctx.add_bound_invoke(
                            inv.name.clone(),
                            InstIdx::UNKNOWN,
                            vec![],
                            inv.copy_span(),
                        );
                    }
                }
                core::Command::Connect(core::Connect { src, dst, .. }) => {
                    let mut check_port = |port: &core::Port| {
                        if let core::PortType::InvPort { invoke, .. } =
                            &port.typ
                        {
                            if ctx.inv_map.get(invoke).is_none() {
                                let err = Error::undefined(
                                    invoke.clone(),
                                    "invocation",
                                )
                                .add_note(diag.add_info(
                                    "unknown invocation",
                                    invoke.copy_span(),
                                ));
                                diag.add_error(err)
                            }
                        } else if let core::PortType::ThisPort(p) = &port.typ {
                            if !ctx.this().ports().iter().any(|pd| pd.name == p)
                            {
                                let err = Error::undefined(p.clone(), "port")
                                    .add_note(diag.add_info(
                                        "unknown port",
                                        p.copy_span(),
                                    ));
                                diag.add_error(err)
                            }
                        }
                    };
                    check_port(src);
                    check_port(dst);
                }
                _ => (),
            }
        }

        if has_errors {
            None
        } else {
            Some(ctx)
        }
    }

    /// Get the **unresolved** signature associated with this component.
    /// If this signature should be completely resolved, use [[InvIdx::resolve_signature]].
    pub fn this(&self) -> &core::Signature {
        self.prog.comp_sig(self.sig)
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
    pub fn get_instance(&self, name: &Id) -> &BoundInstance {
        let idx = self.get_instance_idx(name);
        &self[idx]
    }

    /// Get invocation binding for a given invocation name
    pub fn get_invoke(&self, name: &Id) -> &BoundInvoke {
        let idx = self.get_invoke_idx(name);
        &self[idx]
    }

    /// Get the index for a given instance name
    pub fn get_instance_idx(&self, name: &Id) -> InstIdx {
        self.inst_map[name]
    }

    /// Get the index for a given invocation name
    pub fn get_invoke_idx(&self, name: &Id) -> InvIdx {
        self.inv_map[name]
    }

    /// Add a new instance to this binding.
    /// Returns None when the component is not bound.
    pub fn add_instance(&mut self, inst: &core::Instance) -> InstIdx {
        let sig = self.prog.get_sig_idx(&inst.component);
        self.add_bound_instance(
            inst.name.clone(),
            sig,
            inst.bindings.clone(),
            inst.copy_span(),
        )
    }

    fn add_bound_instance(
        &mut self,
        name: Id,
        sig: SigIdx,
        params: Vec<core::Expr>,
        pos: GPosIdx,
    ) -> InstIdx {
        let idx = InstIdx(self.instances.len());
        self.instances.push(BoundInstance { sig, params, pos });
        self.inst_map.insert(name, idx);
        idx
    }

    /// Add a new invocation to this binding. Fully resolves the binding
    /// by filling in the default arguments.
    /// Returns `None` when the provided instance is not bound.
    pub fn add_invoke(&mut self, inv: &core::Invoke) -> InvIdx {
        let instance = self.inst_map[&inv.instance];
        let events = self
            .prog
            .event_binding(self.instances[instance.0].sig, &inv.abstract_vars)
            .into_iter()
            .map(|b| b.1)
            .collect();
        self.add_bound_invoke(
            inv.name.clone(),
            instance,
            events,
            inv.copy_span(),
        )
    }

    fn add_bound_invoke(
        &mut self,
        name: Id,
        instance: InstIdx,
        events: Vec<Time>,
        pos: GPosIdx,
    ) -> InvIdx {
        let idx = InvIdx(self.invocations.len());
        self.invocations.push(BoundInvoke {
            instance,
            events,
            pos,
        });
        self.inv_map.insert(name, idx);
        idx
    }

    /// Returns a resolved port definition for the given port.
    /// Returns `None` if and only if the given port is a constant.
    pub fn get_resolved_port<F>(
        &self,
        port: &core::Port,
        resolve_liveness: F,
    ) -> Option<core::PortDef>
    where
        F: Fn(
            &core::Range,
            &utils::Binding<Time>,
            &utils::Binding<core::Expr>,
        ) -> core::Range,
    {
        match &port.typ {
            core::PortType::ThisPort(p) => {
                Some(self.prog.comp_sig(self.sig).get_port(p))
            }
            core::PortType::InvPort { invoke, name } => {
                Some(self.get_invoke_idx(invoke).get_invoke_port(
                    self,
                    name,
                    resolve_liveness,
                ))
            }
            core::PortType::Constant(_) => None,
        }
    }
}

/// Signatures bound in a program.
/// Also acts a dispatcher for methods on [[core::Signature]] since external and
/// component signatures have different types.
pub struct ProgBinding<'a> {
    externals: Vec<&'a core::Signature>,
    components: Vec<&'a core::Signature>,
    name_map: HashMap<Id, SigIdx>,
}

impl<'a> ProgBinding<'a> {
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

    fn get_sig_idx(&self, name: &core::Id) -> SigIdx {
        self.find_sig_idx(name)
            .unwrap_or_else(|| panic!("Unknown signature: {}", name))
    }

    /// Add a component signature to the program binding
    pub fn add_component(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::Comp(self.components.len());
        self.components.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    /// Add a component signature to the program binding
    pub fn add_external(&mut self, sig: &'a core::Signature) -> SigIdx {
        let idx = SigIdx::Ext(self.externals.len());
        self.externals.push(sig);
        self.name_map.insert(sig.name.clone(), idx);
        idx
    }

    // ============= Dispatch methods on Signatures =============

    /// Return the underlying signature
    pub fn sig(&self, sig: SigIdx) -> &'a core::Signature {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx],
            SigIdx::Comp(idx) => self.components[idx],
        }
    }

    /// Apply function on either an external or component signature and return the result
    #[inline]
    pub fn map_signature<O>(
        &self,
        sig: SigIdx,
        ext: impl Fn(&'a core::Signature) -> O,
        comp: impl Fn(&'a core::Signature) -> O,
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
    pub fn comp_sig(&self, sig: SigIdx) -> &'a core::Signature {
        match sig {
            SigIdx::Ext(_) => {
                unreachable!("comp_sig called on external signature")
            }
            SigIdx::Comp(idx) => self.components[idx],
        }
    }

    /// Event binding generated from a signature
    /// XXX: Can be constructed using the binding and the event names
    fn event_binding(
        &self,
        sig: SigIdx,
        events: &[Time],
    ) -> utils::Binding<Time> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx].event_binding(events),
            SigIdx::Comp(idx) => self.components[idx].event_binding(events),
        }
    }

    /// XXX: Can be constructed using the binding and the param names
    fn param_binding(
        &self,
        sig: SigIdx,
        params: &[core::Expr],
    ) -> utils::Binding<core::Expr> {
        match sig {
            SigIdx::Ext(idx) => self.externals[idx].param_binding(params),
            SigIdx::Comp(idx) => self.components[idx].param_binding(params),
        }
    }
}

impl<'a> From<&'a core::Namespace> for ProgBinding<'a> {
    fn from(ns: &'a core::Namespace) -> Self {
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
