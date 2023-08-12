use super::sig_map::Sig;
use super::{BuildRes, ScopeMap, SigMap};
use crate::ast::{self, Id};
use crate::errors::Error;
use crate::ir::{self, Ctx, DenseIndexInfo, PortIdx};
use crate::{diagnostics, utils};
use std::rc::Rc;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
/// A custom struct used to index ports
pub(super) enum InvPort {
    Sig(ir::Direction, Id),
    Inv(ir::InvIdx, ir::Direction, Id),
    Local(Id),
}

impl InvPort {
    fn name(&self) -> &Id {
        match self {
            Self::Sig(_, id) | Self::Inv(_, _, id) | Self::Local(id) => id,
        }
    }
}

impl std::fmt::Display for InvPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sig(dir, id) => write!(f, "{}({})", dir, id),
            Self::Inv(inv, dir, id) => write!(f, "{}({} in {})", dir, id, inv),
            Self::Local(id) => write!(f, "{}", id),
        }
    }
}

/// Context used while building the IR.
pub(super) struct BuildCtx<'prog> {
    // Mapping from names of instance to (<parameter bindings>, <component name>).
    // We keep around the parameter bindings as [ast::Expr] because we need to resolve
    // port definition in invokes using them.
    pub inst_to_sig: DenseIndexInfo<
        ir::Instance,
        (Rc<utils::Binding<ast::Expr>>, ast::Loc<Id>),
    >,

    /// The current component we're building up
    comp: ir::Component,

    /// Current diagnostics context
    diag: diagnostics::Diagnostics,

    /// Map of currently defined signatures
    sigs: &'prog SigMap,

    // Mapping from names to IR nodes.
    event_map: ScopeMap<ir::Event>,
    inst_map: ScopeMap<ir::Instance>,
    inv_map: ScopeMap<ir::Invoke>,
    port_map: ScopeMap<ir::Port, InvPort>,
    param_map: ScopeMap<ir::Param>,

    /// Index for generating unique names
    name_idx: u32,
}

impl<'prog> BuildCtx<'prog> {
    pub fn new(comp: ir::Component, sigs: &'prog SigMap) -> Self {
        Self {
            comp,
            sigs,
            diag: diagnostics::Diagnostics::default(),
            name_idx: 0,
            param_map: ScopeMap::new(),
            event_map: ScopeMap::new(),
            port_map: ScopeMap::new(),
            inst_map: ScopeMap::new(),
            inv_map: ScopeMap::new(),
            inst_to_sig: DenseIndexInfo::default(),
        }
    }

    pub fn diag(&mut self) -> &mut diagnostics::Diagnostics {
        &mut self.diag
    }

    #[inline]
    /// Get a mutable reference to current component
    pub fn comp(&mut self) -> &mut ir::Component {
        &mut self.comp
    }

    #[inline]
    // takes the component from the builder
    pub fn take(self) -> ir::Component {
        self.comp
    }

    /// Generate a unique, new name for unused parameters
    pub fn gen_name(&mut self) -> Id {
        let name = format!("_{}", self.name_idx);
        self.name_idx += 1;
        Id::from(name)
    }

    /// Push a new scope level
    #[inline]
    fn push(&mut self) {
        self.param_map.push();
        self.event_map.push();
        self.port_map.push();
        self.inst_map.push();
        self.inv_map.push();
    }

    /// Pop the last scope level
    #[inline]
    fn pop(&mut self) {
        self.param_map.pop();
        self.event_map.pop();
        self.port_map.pop();
        self.inst_map.pop();
        self.inv_map.pop();
    }

    pub fn try_with_scope<T, F>(&mut self, f: F) -> BuildRes<T>
    where
        F: FnOnce(&mut Self) -> BuildRes<T>,
    {
        self.push();
        let out = f(self);
        self.pop();
        out
    }

    /// Get the signature if bound or return an error
    pub fn get_sig(&mut self, id: &ast::Loc<Id>) -> BuildRes<&'prog Sig> {
        let name = id.inner();
        match self.sigs.get(name) {
            Some(s) => Ok(s),
            None => {
                let diag = &mut self.diag;
                let undef = Error::undefined(*name, "signature").add_note(
                    diag.add_info(
                        format!("signature `{id}' is not defined"),
                        id.pos(),
                    ),
                );
                diag.add_error(undef);
                Err(std::mem::take(diag))
            }
        }
    }

    /// Get a signature from the component index and panic if its not found.
    pub fn sig_from_idx(&self, idx: ir::CompIdx) -> &Sig {
        self.sigs.get_idx(idx).unwrap()
    }

    pub fn set_sig_map(&mut self, sigs: &'prog SigMap) {
        self.sigs = sigs;
    }

    pub fn get_param(&mut self, id: &Id) -> BuildRes<ir::ParamIdx> {
        let name = id;
        match self.param_map.get(name) {
            Some(p) => Ok(*p),
            None => {
                let diag = &mut self.diag;
                let undef =
                    Error::undefined(format!("#{}", *name).into(), "parameter");
                // .add_note(
                //     diag.add_info(
                //         format!("parameter `{id}' is not defined"),
                //         id.pos(),
                //     ),
                // );
                diag.add_error(undef);
                Err(std::mem::take(diag))
            }
        }
    }

    pub fn add_param(&mut self, name: Id, param: ir::ParamIdx) {
        self.param_map.insert(name, param);
    }

    /// Add a new port to the ctx
    pub fn add_port(&mut self, name: Id, port: PortIdx) {
        let owner = self.comp.get(port).owner.clone();
        let owner = match owner {
            ir::PortOwner::Sig { dir } => InvPort::Sig(dir, name),
            ir::PortOwner::Inv { inv, dir, .. } => InvPort::Inv(inv, dir, name),
            ir::PortOwner::Local => InvPort::Local(name),
        };
        self.port_map.insert(owner, port);
    }

    /// Find a port and return None if it is not found
    pub fn find_port(&mut self, port: &InvPort) -> Option<PortIdx> {
        self.port_map.get(port).copied()
    }

    /// Get a port and panic out if it is not found
    pub fn get_port(&mut self, port: &InvPort) -> PortIdx {
        // Clone here because we use owner in the error message below
        if let Some(idx) = self.find_port(port) {
            return idx;
        }

        let name = port.name();

        // We are going to error out anyways so attempt to find the scope for
        // *any* port with the same name but a different port owner.
        let other = self
            .port_map
            .as_flat_vec()
            .into_iter()
            .find_map(|(p, _)| if p.name() == name { Some(p) } else { None });
        let mut msg = format!("Port `{port}' not found");
        if let Some(port) = other {
            msg.push_str(&format!(
                ". However, a port with the same name exists: `{port}'",
            ));
        }
        msg.push_str(&format!(
            ". Component:\n{comp}",
            comp = ir::Printer::comp_str(&self.comp)
        ));
        unreachable!("{msg}")
    }

    pub fn add_inst(&mut self, name: Id, inst: ir::InstIdx) {
        self.inst_map.insert(name, inst);
    }

    pub fn get_inst(&mut self, id: &ast::Loc<Id>) -> BuildRes<ir::InstIdx> {
        let name = *id.inner();
        match self.inst_map.get(&name) {
            Some(idx) => Ok(*idx),
            None => {
                let diag = &mut self.diag;
                let undef =
                    Error::undefined(name, "instance").add_note(diag.add_info(
                        format!("instance `{name}' is not defined"),
                        id.pos(),
                    ));
                diag.add_error(undef);
                Err(std::mem::take(diag))
            }
        }
    }

    pub fn add_inv(&mut self, name: Id, inv: ir::InvIdx) {
        self.inv_map.insert(name, inv);
    }

    pub fn get_inv(&mut self, id: &ast::Loc<Id>) -> BuildRes<ir::InvIdx> {
        let name = *id.inner();
        match self.inv_map.get(&name) {
            Some(idx) => Ok(*idx),
            None => {
                let diag = &mut self.diag;
                let undef = Error::undefined(name, "invocation").add_note(
                    diag.add_info(
                        format!("invocation `{name}' is not defined"),
                        id.pos(),
                    ),
                );
                diag.add_error(undef);
                Err(std::mem::take(diag))
            }
        }
    }

    pub fn add_event(&mut self, name: Id, event: ir::EventIdx) {
        self.event_map.insert(name, event);
    }

    pub fn get_event(&mut self, id: &Id) -> BuildRes<ir::EventIdx> {
        let name = *id;
        match self.event_map.get(&name) {
            Some(idx) => Ok(*idx),
            None => {
                let diag = &mut self.diag;
                let undef = Error::undefined(name, "event");
                // .add_note(diag.add_info(
                //     format!("event `{name}' is not defined"),
                //     id.pos(),
                // ));
                diag.add_error(undef);
                Err(std::mem::take(diag))
            }
        }
    }
}
