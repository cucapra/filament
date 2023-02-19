//! Context that tracks the binding information in a particular program
use super::{BoundInstance, BoundInvoke, InstIdx, InvIdx, ProgBinding, SigIdx};
use crate::{
    core::{self, Id, Time},
    diagnostics,
    errors::{Error, WithPos},
    utils::{self, GPosIdx},
};
use std::collections::HashMap;

pub(super) struct BoundComponent {
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

impl BoundComponent {
    /// Add a new instance to this binding.
    /// Returns None when the component is not bound.
    pub fn add_instance(
        &mut self,
        prog: &ProgBinding,
        inst: &core::Instance,
    ) -> InstIdx {
        let sig = prog.get_sig_idx(&inst.component);
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
        let idx = InstIdx::new(self.instances.len());
        self.instances.push(BoundInstance::new(sig, params, pos));
        self.inst_map.insert(name, idx);
        idx
    }

    /// Add a new invocation to this binding. Fully resolves the binding
    /// by filling in the default arguments.
    /// Returns `None` when the provided instance is not bound.
    pub fn add_invoke(
        &mut self,
        prog: &ProgBinding,
        inv: &core::Invoke,
    ) -> InvIdx {
        let inst_idx = self.inst_map[&inv.instance];
        let instance = &self[inst_idx];
        let events = prog[instance.sig]
            .event_binding(&inv.abstract_vars)
            .into_iter()
            .map(|b| b.1)
            .collect();
        self.add_bound_invoke(
            inv.name.clone(),
            inst_idx,
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
        let idx = InvIdx::new(self.invocations.len());
        self.invocations
            .push(BoundInvoke::new(instance, events, pos));
        self.inv_map.insert(name, idx);
        idx
    }

    fn process_cmds(&mut self, prog: &ProgBinding, cmds: &Vec<core::Command>) {
        for cmd in cmds {
            match cmd {
                core::Command::Instance(inst) => {
                    self.add_instance(prog, inst);
                }
                core::Command::Invoke(inv) => {
                    self.add_invoke(prog, inv);
                }
                core::Command::ForLoop(l) => {
                    self.process_cmds(prog, &l.body);
                }
                core::Command::Connect(_) | core::Command::Fsm(_) => (),
            }
        }
    }

    fn process_checked_cmds(
        &mut self,
        prog: &ProgBinding,
        cmds: &Vec<core::Command>,
        diag: &mut diagnostics::Diagnostics,
    ) -> bool {
        let mut has_errors = false;
        for cmd in cmds {
            match cmd {
                core::Command::Instance(inst) => {
                    let comp = &inst.component;
                    if prog.find_sig_idx(comp).is_some() {
                        self.add_instance(prog, inst);
                    } else {
                        has_errors = true;
                        // If there is no component with this name, add an error and use a dummy signature
                        let err = Error::undefined(comp.clone(), "component")
                            .add_note(diag.add_info(
                                "unknown component",
                                comp.copy_span(),
                            ));
                        diag.add_error(err);
                        self.add_bound_instance(
                            inst.name.clone(),
                            SigIdx::UNKNOWN,
                            vec![],
                            inst.copy_span(),
                        );
                    }
                }
                core::Command::Invoke(inv) => {
                    if self.inst_map.get(&inv.instance).is_some() {
                        // If there have been previous errors, we cannot rely on signatures being valid
                        if has_errors {
                            self.add_bound_invoke(
                                inv.name.clone(),
                                InstIdx::UNKNOWN,
                                vec![],
                                inv.copy_span(),
                            );
                        } else {
                            self.add_invoke(prog, inv);
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
                        self.add_bound_invoke(
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
                            if self.inv_map.get(invoke).is_none() {
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
                            if !prog[self.sig]
                                .ports()
                                .iter()
                                .any(|pd| pd.name == p)
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
                core::Command::ForLoop(l) => {
                    has_errors |=
                        self.process_checked_cmds(prog, &l.body, diag);
                }
                core::Command::Fsm(_) => (),
            }
        }

        has_errors
    }
}

// Index into the component binding using InstIdx
impl std::ops::Index<InstIdx> for BoundComponent {
    type Output = BoundInstance;
    fn index(&self, idx: InstIdx) -> &Self::Output {
        &self.instances[idx.get()]
    }
}

// Index into the component binding using InvIdx
impl std::ops::Index<InvIdx> for BoundComponent {
    type Output = BoundInvoke;
    fn index(&self, idx: InvIdx) -> &Self::Output {
        &self.invocations[idx.get()]
    }
}

/// Track binding information for a component
pub struct CompBinding<'p> {
    /// Context associated with the program
    pub prog: &'p ProgBinding<'p>,
    comp: BoundComponent,
}

impl<'p> CompBinding<'p> {
    /// Construct a new binding context for a component
    pub fn new(prog_ctx: &'p ProgBinding<'p>, comp: &core::Component) -> Self {
        Self::from_component(prog_ctx, &comp.sig.name, &comp.body)
    }

    /// Construct bindings while checking for errors.
    /// Returns None if there are any errors.
    pub fn new_checked(
        prog_ctx: &'p ProgBinding<'p>,
        comp: &core::Component,
        diag: &mut diagnostics::Diagnostics,
    ) -> Option<Self> {
        Self::from_component_checked(prog_ctx, &comp.sig.name, &comp.body, diag)
    }

    /// Construct a new instance using information from a [core::Component].
    pub fn from_component(
        prog: &'p ProgBinding<'p>,
        comp: &core::Id,
        cmds: &Vec<core::Command>,
    ) -> Self {
        let sig = prog.get_sig_idx(comp);
        let mut comp = BoundComponent {
            sig,
            instances: Vec::new(),
            invocations: Vec::new(),
            inst_map: HashMap::new(),
            inv_map: HashMap::new(),
        };
        comp.process_cmds(prog, cmds);
        Self { prog, comp }
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
        let mut comp = BoundComponent {
            sig,
            instances: Vec::new(),
            invocations: Vec::new(),
            inst_map: HashMap::new(),
            inv_map: HashMap::new(),
        };
        let has_errors = comp.process_checked_cmds(prog, cmds, diag);
        if has_errors {
            None
        } else {
            Some(Self { prog, comp })
        }
    }

    /// Get the **unresolved** signature associated with this component.
    /// If this signature should be completely resolved, use [[InvIdx::resolve_signature]].
    pub fn this(&self) -> &core::Signature {
        &self.prog[self.comp.sig]
    }

    /// Return instances associated with this component
    pub fn instances(&self) -> impl Iterator<Item = InstIdx> {
        (0..self.comp.instances.len()).map(InstIdx::new)
    }

    /// Return the invocations associated with this component
    pub fn invocations(&self) -> impl Iterator<Item = InvIdx> {
        (0..self.comp.invocations.len()).map(InvIdx::new)
    }

    /// Signature associated with this component
    pub fn sig(&self) -> SigIdx {
        self.comp.sig
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
        *self
            .comp
            .inst_map
            .get(name)
            .unwrap_or_else(|| panic!("Unknown instance: {name}"))
    }

    /// Get the index for a given invocation name
    pub fn get_invoke_idx(&self, name: &Id) -> InvIdx {
        *self
            .comp
            .inv_map
            .get(name)
            .unwrap_or_else(|| panic!("Unknown invocation: {name}"))
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
            core::PortType::ThisPort(p) => Some(self.this().get_port(p)),
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

// Index into the component binding using InstIdx
impl<'p> std::ops::Index<InstIdx> for CompBinding<'p> {
    type Output = BoundInstance;
    fn index(&self, idx: InstIdx) -> &Self::Output {
        &self.comp[idx]
    }
}

// Index into the component binding using InvIdx
impl<'p> std::ops::Index<InvIdx> for CompBinding<'p> {
    type Output = BoundInvoke;
    fn index(&self, idx: InvIdx) -> &Self::Output {
        &self.comp[idx]
    }
}
