//! Context that tracks the binding information in a particular program
use super::{BoundInstance, BoundInvoke, InstIdx, InvIdx, ProgBinding, SigIdx};
use crate::{
    core::{self, Id, Loc, Time},
    diagnostics,
    errors::Error,
    idx,
    utils::GPosIdx,
};
use itertools::Itertools;
use std::collections::HashMap;

pub type BundleIdx = idx!(core::Bundle);

pub struct BoundComponent {
    /// Signature associated with this component
    sig: SigIdx,
    /// Instances bound in this component
    instances: Vec<BoundInstance>,
    /// Invocations bound in this component
    invocations: Vec<BoundInvoke>,
    /// Bundles bound in the component
    bundles: Vec<core::Bundle>,
    /// Mapping from name of instance to its index
    inst_map: HashMap<Id, InstIdx>,
    /// Mapping from name of invocation to its index
    inv_map: HashMap<Id, InvIdx>,
    /// Mapping from name of bundle to its index
    bundle_map: HashMap<Id, BundleIdx>,
    /// Error was encountered while binding this component
    is_err: bool,
}

impl From<SigIdx> for BoundComponent {
    fn from(sig: SigIdx) -> Self {
        Self {
            sig,
            instances: Vec::new(),
            invocations: Vec::new(),
            bundles: Vec::new(),
            inst_map: HashMap::new(),
            inv_map: HashMap::new(),
            bundle_map: HashMap::new(),
            is_err: false,
        }
    }
}

impl BoundComponent {
    /// We encountered an error while binding this component
    fn set_err(&mut self) {
        log::info!("Error encountered while binding component");
        self.is_err = true;
    }

    /// Check if this name is already bound in this component
    fn name_is_bound(&self, name: &Id) -> Option<GPosIdx> {
        if let Some(idx) = self.inst_map.get(name) {
            Some(self.instances[idx.get()].pos)
        } else if let Some(bun) = self.bundle_map.get(name) {
            Some(self.bundles[bun.get()].name.pos())
        } else {
            self.inv_map
                .get(name)
                .map(|idx| self.invocations[idx.get()].pos)
        }
    }

    /// Add a new instance to this binding.
    /// Returns None when the component is not bound.
    pub fn add_instance(
        &mut self,
        prog: &ProgBinding,
        inst: &core::Instance,
    ) -> InstIdx {
        let sig = prog.get_sig_idx(&inst.component);
        self.add_bound_instance(
            *inst.name.inner(),
            sig,
            inst.bindings.clone(),
            inst.name.pos(),
        )
    }

    /// Add a new bundle to this binding.
    pub fn add_bundle(&mut self, bundle: core::Bundle) -> BundleIdx {
        let idx = BundleIdx::new(self.bundles.len());
        self.bundle_map.insert(*bundle.name.inner(), idx);
        self.bundles.push(bundle);
        idx
    }

    /// Build a binding from a component's body.
    /// Assumes that there are no binding errors in the body.
    pub fn from_component(
        prog: &ProgBinding,
        name: &core::Id,
        cmds: &Vec<core::Command>,
    ) -> Self {
        let idx = prog.get_sig_idx(name);
        let mut bind = BoundComponent::from(idx);
        bind.process_cmds(prog, cmds);
        bind
    }

    fn add_bound_instance(
        &mut self,
        name: Id,
        sig: SigIdx,
        params: Vec<core::Loc<core::Expr>>,
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
        let inst_idx = self.inst_map[inv.instance.inner()];
        let instance = &self[inst_idx];
        let abs_vars = &inv.abstract_vars;
        let events = prog[instance.sig]
            .event_binding(inv.abstract_vars.iter().map(|c| c.inner().clone()))
            .into_iter()
            .enumerate()
            .map(|(idx, b)| {
                if let Some(ev) = abs_vars.get(idx) {
                    Loc::new(b.1, ev.pos())
                } else {
                    Loc::unknown(b.1)
                }
            })
            .collect();
        self.add_bound_invoke(
            *inv.name.inner(),
            inst_idx,
            events,
            inv.abstract_vars.len(),
            inv.name.pos(),
        )
    }

    fn add_bound_invoke(
        &mut self,
        name: Id,
        instance: InstIdx,
        events: Vec<Loc<Time>>,
        default_start: usize,
        pos: GPosIdx,
    ) -> InvIdx {
        let idx = InvIdx::new(self.invocations.len());
        self.invocations.push(BoundInvoke::new(
            instance,
            events,
            default_start,
            pos,
        ));
        self.inv_map.insert(name, idx);
        idx
    }

    pub(super) fn process_cmds(
        &mut self,
        prog: &ProgBinding,
        cmds: &Vec<core::Command>,
    ) {
        for cmd in cmds {
            match cmd {
                core::Command::Instance(inst) => {
                    self.add_instance(prog, inst);
                }
                core::Command::Invoke(inv) => {
                    self.add_invoke(prog, inv);
                }
                core::Command::Bundle(bundle) => {
                    self.add_bundle(bundle.clone());
                }
                core::Command::ForLoop(l) => {
                    self.process_cmds(prog, &l.body);
                }
                core::Command::If(if_) => {
                    self.process_cmds(prog, &if_.then);
                    self.process_cmds(prog, &if_.alt);
                }
                core::Command::Connect(_)
                | core::Command::Fsm(_)
                | core::Command::Fact(_) => (),
            }
        }
    }

    pub(super) fn process_checked_cmds(
        &mut self,
        prog: &ProgBinding,
        cmds: &Vec<core::Command>,
        diag: &mut diagnostics::Diagnostics,
    ) {
        for cmd in cmds {
            match cmd {
                core::Command::Instance(inst) => {
                    let comp = &inst.component;
                    // Check if the name is already bound
                    if let Some(pos) = self.name_is_bound(&inst.name) {
                        self.set_err();
                        let err =
                            Error::already_bound(
                                *inst.name.inner(),
                                "instance",
                            )
                            .add_note(diag.add_info(
                                "name is already bound",
                                inst.name.pos(),
                            ))
                            .add_note(diag.add_info("previous binding", pos));
                        diag.add_error(err);
                    }
                    if prog.find_sig_idx(comp).is_some() {
                        self.add_instance(prog, inst);
                    } else {
                        self.set_err();
                        // If there is no component with this name, add an error and use a dummy signature
                        let err = Error::undefined(*comp.inner(), "component")
                            .add_note(
                                diag.add_info("unknown component", comp.pos()),
                            );
                        diag.add_error(err);
                        self.add_bound_instance(
                            *inst.name.inner(),
                            SigIdx::UNKNOWN,
                            vec![],
                            inst.name.pos(),
                        );
                    }
                }
                core::Command::Invoke(inv) => {
                    // Check if the invoke name is already bound
                    if let Some(pos) = self.name_is_bound(&inv.name) {
                        self.set_err();
                        let err =
                            Error::already_bound(*inv.name.inner(), "invoke")
                                .add_note(diag.add_info(
                                    "name is already bound",
                                    inv.name.pos(),
                                ))
                                .add_note(
                                    diag.add_info("previous binding", pos),
                                );
                        diag.add_error(err);
                    }

                    // Check if the instance name is bound
                    if self.inst_map.get(&inv.instance).is_some() {
                        // If there have been previous errors, we cannot rely on signatures being valid
                        if self.is_err {
                            self.add_bound_invoke(
                                *inv.name.inner(),
                                InstIdx::UNKNOWN,
                                vec![],
                                0,
                                inv.name.pos(),
                            );
                        } else {
                            self.add_invoke(prog, inv);
                        }
                    } else {
                        self.set_err();
                        // If there is no component with this name, add an error and use a dummy signature
                        let err =
                            Error::undefined(*inv.instance.inner(), "instance")
                                .add_note(diag.add_info(
                                    "unknown instance",
                                    inv.instance.pos(),
                                ));
                        diag.add_error(err);
                        self.add_bound_invoke(
                            *inv.name.inner(),
                            InstIdx::UNKNOWN,
                            vec![],
                            0,
                            inv.name.pos(),
                        );
                    }
                }
                core::Command::Connect(core::Connect { src, dst, .. }) => {
                    let mut check_port = |port: &core::Port| {
                        if let core::Port::InvPort { invoke, .. } = &port {
                            if self.inv_map.get(invoke).is_none() {
                                let err = Error::undefined(
                                    *invoke.inner(),
                                    "invocation",
                                )
                                .add_note(diag.add_info(
                                    "unknown invocation",
                                    invoke.pos(),
                                ));
                                diag.add_error(err)
                            }
                        } else if let core::Port::This(p) = &port {
                            if !prog[self.sig]
                                .ports()
                                .iter()
                                .any(|pd| pd.name() == p)
                            {
                                let err = Error::undefined(*p.inner(), "port")
                                    .add_note(
                                        diag.add_info("unknown port", p.pos()),
                                    );
                                diag.add_error(err)
                            }
                        }
                    };
                    check_port(src);
                    check_port(dst);
                }
                core::Command::ForLoop(l) => {
                    self.process_checked_cmds(prog, &l.body, diag);
                }
                core::Command::If(i) => {
                    self.process_checked_cmds(prog, &i.then, diag);
                    self.process_checked_cmds(prog, &i.alt, diag);
                }
                core::Command::Bundle(bl) => {
                    self.add_bundle(bl.clone());
                }
                core::Command::Fsm(_) | core::Command::Fact(_) => (),
            }
        }
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

impl std::ops::Index<BundleIdx> for BoundComponent {
    type Output = core::Bundle;
    fn index(&self, idx: BundleIdx) -> &Self::Output {
        &self.bundles[idx.get()]
    }
}

/// Track binding information for a component
pub struct CompBinding<'c, 'p: 'c> {
    /// Context associated with the program
    pub prog: &'p ProgBinding<'p>,
    pub comp: &'c BoundComponent,
}

impl<'c, 'p> CompBinding<'c, 'p> {
    /// Construct a new binding context for a component
    pub fn new(prog: &'p ProgBinding<'p>, name: &Id) -> Self {
        let comp = prog.get_comp_binding(name);
        Self { prog, comp }
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

    /// Get the bundle associated with a given bundle name
    pub fn get_bundle_idx(&self, name: &Id) -> BundleIdx {
        *self.comp.bundle_map.get(name).unwrap_or_else(|| {
            panic!(
                "Unknown bundle: {name}. Known bundles: [{}]",
                self.comp
                    .bundle_map
                    .keys()
                    .map(|id| id.to_string())
                    .collect_vec()
                    .join(", ")
            )
        })
    }

    fn bundle_access(
        mut bun: core::Bundle,
        acc: &core::Access,
    ) -> core::PortDef {
        match acc {
            core::Access::Index(idx) => bun.index(idx.clone()),
            core::Access::Range { start, end } => {
                bun.typ = bun.typ.shrink(start.clone(), end.clone());
                bun.into()
            }
        }
    }

    /// Returns a resolved port definition for the given port.
    /// Returns `None` if and only if the given port is a constant.
    pub fn get_resolved_port(
        &self,
        port: &core::Port,
    ) -> Option<core::PortDef> {
        match &port {
            core::Port::This(p) => Some(self.this().get_port(p.inner()).take()),
            core::Port::InvPort { invoke, name } => {
                Some(self.get_invoke_idx(invoke).resolved_inv_port(self, name))
            }
            core::Port::Bundle { name, access, .. } => {
                let bi = self.get_bundle_idx(name);
                Some(Self::bundle_access(self[bi].clone(), access.inner()))
            }
            core::Port::Constant(_) => None,
            core::Port::InvBundle {
                invoke,
                port,
                access,
            } => {
                let port =
                    self.get_invoke_idx(invoke).resolved_inv_port(self, port);
                let core::PortDef::Bundle(bun) = port else {
                    unreachable!("Expected bundle port, received: `{port}'")
                };
                Some(Self::bundle_access(bun, access))
            }
        }
    }
}

// Index into the component binding using InstIdx
impl<'c, 'p> std::ops::Index<InstIdx> for CompBinding<'c, 'p> {
    type Output = BoundInstance;
    fn index(&self, idx: InstIdx) -> &Self::Output {
        &self.comp[idx]
    }
}

// Index into the component binding using InvIdx
impl<'c, 'p> std::ops::Index<InvIdx> for CompBinding<'c, 'p> {
    type Output = BoundInvoke;
    fn index(&self, idx: InvIdx) -> &Self::Output {
        &self.comp[idx]
    }
}

impl<'c, 'p> std::ops::Index<BundleIdx> for CompBinding<'c, 'p> {
    type Output = core::Bundle;
    fn index(&self, idx: BundleIdx) -> &Self::Output {
        &self.comp[idx]
    }
}
