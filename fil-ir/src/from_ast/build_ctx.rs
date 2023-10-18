use super::sig_map::Sig;
use super::{BuildRes, ScopeMap, SigMap};
use crate::{self as ir, Ctx, DenseIndexInfo, PortIdx};
use fil_ast as ast;
use fil_utils::{self as utils, Error, Id};
use std::rc::Rc;
use utils::InfoIdx;

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
/// A custom struct used to index ports
pub(super) enum OwnedPort {
    Sig(ir::Direction, ast::Loc<Id>),
    Inv(ir::InvIdx, ir::Direction, ast::Loc<Id>),
    Local(ast::Loc<Id>),
}

impl OwnedPort {
    fn name(&self) -> Id {
        match self {
            Self::Sig(_, id) | Self::Inv(_, _, id) | Self::Local(id) => {
                *id.inner()
            }
        }
    }

    fn pos(&self) -> utils::GPosIdx {
        match self {
            Self::Sig(_, id) | Self::Inv(_, _, id) | Self::Local(id) => {
                id.pos()
            }
        }
    }
}

impl std::fmt::Display for OwnedPort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sig(dir, id) => write!(f, "{}({})", dir, id),
            Self::Inv(inv, dir, id) => write!(f, "{}({} in {})", dir, id, inv),
            Self::Local(id) => write!(f, "{}", id),
        }
    }
}

#[derive(Eq, PartialEq, Hash, Clone, Debug)]
pub(super) enum OwnedParam {
    /// A parameter defiend by an instance.
    Instance(ir::InstIdx, ast::Id),
    /// A parameter defined locally in the component (either the signature, a
    /// let binding, or an exists binding).
    Local(ast::Id),
}

impl OwnedParam {
    pub fn local(id: ast::Id) -> Self {
        Self::Local(id)
    }

    pub fn inst(inst: ir::InstIdx, id: ast::Id) -> Self {
        Self::Instance(inst, id)
    }

    pub fn param_owner(id: ast::Id, owner: &ir::ParamOwner) -> Self {
        match owner {
            ir::ParamOwner::Sig
            | ir::ParamOwner::Exists { .. }
            | ir::ParamOwner::Bundle(_)
            | ir::ParamOwner::Loop => Self::Local(id),
            ir::ParamOwner::Let { .. } => todo!(),
            ir::ParamOwner::Instance { inst, .. } => Self::Instance(*inst, id),
        }
    }
}

impl std::fmt::Display for OwnedParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwnedParam::Instance(inst, name) => write!(f, "{inst}::{name}"),
            OwnedParam::Local(id) => write!(f, "{}", id),
        }
    }
}

/// Context used while building the IR.
pub(super) struct BuildCtx<'prog> {
    /// Mapping from names of instance to (<parameter bindings>, <component name>).
    /// We keep around the parameter bindings as [ast::Expr] because we need to resolve
    /// port definition in invokes using them.
    pub inst_to_sig: DenseIndexInfo<
        ir::Instance,
        (Rc<ast::Binding<ast::Expr>>, ast::Loc<Id>),
    >,

    /// The current component we're building up
    comp: ir::Component,

    /// Current diagnostics context
    diag: utils::Diagnostics,

    /// Map of currently defined signatures
    sigs: &'prog SigMap,

    // Mapping from names to IR nodes.
    event_map: ScopeMap<ir::EventIdx>,
    inst_map: ScopeMap<ir::InstIdx>,
    inv_map: ScopeMap<ir::InvIdx>,
    // The port map is indexed using an owner representation because they can be
    // defined by several constructs.
    port_map: ScopeMap<ir::PortIdx, OwnedPort>,

    /// The parameter map returns instead of a [ir::ParamIdx] because let-bound
    /// parameters are immediately rewritten.
    param_map: ScopeMap<ir::ExprIdx, OwnedParam>,
}

impl<'prog> BuildCtx<'prog> {
    pub fn new(comp: ir::Component, sigs: &'prog SigMap) -> Self {
        Self {
            comp,
            sigs,
            diag: utils::Diagnostics::default(),
            param_map: ScopeMap::new(),
            event_map: ScopeMap::new(),
            port_map: ScopeMap::new(),
            inst_map: ScopeMap::new(),
            inv_map: ScopeMap::new(),
            inst_to_sig: DenseIndexInfo::default(),
        }
    }

    pub fn diag(&mut self) -> &mut utils::Diagnostics {
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

    /// Fail the current computation and return the error
    pub fn fail<T>(
        &mut self,
        err: Error,
        info: impl IntoIterator<Item = InfoIdx>,
    ) -> BuildRes<T> {
        let mut err = err;
        for info in info.into_iter() {
            err = err.add_note(info);
        }
        self.diag.add_error(err);
        Err(std::mem::take(&mut self.diag))
    }

    /// Get the signature if bound or return an error
    pub fn get_sig(&mut self, id: &ast::Loc<Id>) -> BuildRes<&'prog Sig> {
        let name = id.inner();
        match self.sigs.get(name) {
            Some(s) => Ok(s),
            None => {
                let info = self.diag.add_info(
                    format!("signature `{id}' is not defined"),
                    id.pos(),
                );
                self.fail(Error::undefined(*name, "signature"), [info])
            }
        }
    }

    /// Update the signature map
    pub fn set_sig_map(&mut self, sigs: &'prog SigMap) {
        self.sigs = sigs;
    }

    /// Add a let-bound parameter's as binding as the thing it should resolve to.
    /// This has the effect of inlining all let-bound parameters.
    pub fn add_let_param(&mut self, id: Id, expr: ir::ExprIdx) {
        self.param_map.insert(OwnedParam::Local(id), expr);
    }

    /// Get the expression binding for a parameter.
    pub fn get_param(
        &mut self,
        param: &OwnedParam,
        pos: utils::GPosIdx,
    ) -> BuildRes<ir::ExprIdx> {
        match self.param_map.get(param) {
            Some(p) => Ok(*p),
            None => {
                let info = self.diag.add_info("unknown parameter", pos);
                self.fail(
                    Error::undefined(format!("{}", param), "parameter"),
                    [info],
                )
            }
        }
    }

    /// Add a parameter to the current map.
    pub fn add_param_map(&mut self, owner: OwnedParam, param: ir::ExprIdx) {
        self.param_map.insert(owner, param);
    }

    /// Add a new port to the ctx
    pub fn add_port(&mut self, name: ast::Loc<Id>, port: PortIdx) {
        let owner = self.comp.get(port).owner.clone();
        let owner = match owner {
            ir::PortOwner::Sig { dir } => OwnedPort::Sig(dir, name),
            ir::PortOwner::Inv { inv, dir, .. } => {
                OwnedPort::Inv(inv, dir, name)
            }
            ir::PortOwner::Local => OwnedPort::Local(name),
        };
        self.port_map.insert(owner, port);
    }

    /// Find a port and return None if it is not found
    pub fn find_port(&mut self, port: &OwnedPort) -> Option<PortIdx> {
        self.port_map.get(port).copied()
    }

    /// Get a port and panic out if it is not found
    pub fn get_port(&mut self, port: &OwnedPort) -> BuildRes<PortIdx> {
        if let Some(idx) = self.find_port(port) {
            return Ok(idx);
        }

        // We are going to error out anyways so attempt to find the scope for
        // *any* port with the same name but a different port owner.
        let name = port.name();
        let other = self
            .port_map
            .as_flat_vec()
            .into_iter()
            .find_map(|(p, _)| if p.name() == name { Some(p) } else { None });
        let mut err = Error::undefined(name, "port");
        err =
            err.add_note(self.diag.add_info(
                format!("`{}' is not a defined port", name),
                port.pos(),
            ));
        if let Some(port) = other {
            err = err.add_note(self.diag.add_info(
                "however, a port with the same name exists",
                port.pos(),
            ));
        }
        self.fail(err, [])
    }

    pub fn add_inst(&mut self, name: Id, inst: ir::InstIdx) {
        self.inst_map.insert(name, inst);
    }

    pub fn get_inst(&mut self, id: &ast::Loc<Id>) -> BuildRes<ir::InstIdx> {
        let name = *id.inner();
        match self.inst_map.get(&name) {
            Some(idx) => Ok(*idx),
            None => {
                let info = self.diag.add_info(
                    format!("instance `{name}' is not defined"),
                    id.pos(),
                );
                self.fail(Error::undefined(name, "instance"), [info])
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
                let info = self.diag.add_info(
                    format!("invocation `{name}' is not defined"),
                    id.pos(),
                );
                self.fail(Error::undefined(name, "invocation"), [info])
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
                self.fail(Error::undefined(name, "event"), [])
                // .add_note(diag.add_info(
                //     format!("event `{name}' is not defined"),
                //     id.pos(),
                // ));
            }
        }
    }
}
