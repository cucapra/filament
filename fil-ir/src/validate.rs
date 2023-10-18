use super::DisplayCtx;
use crate::{self as ir, Ctx, MutCtx};
/// Validate the current context
/// If calling the methods in this does not result in a panic, then the corresponding IR structure is valid.
/// The validity condition for each structure is defined in the corresponding method.
pub struct Validate<'a> {
    /// The context for the program being evaluated
    ctx: &'a ir::Context,
    comp: &'a ir::Component,
}

impl<'a> Validate<'a> {
    /// Check a context
    pub fn context(ctx: &'a ir::Context) {
        for (_, comp) in ctx.iter() {
            Self { ctx, comp }.comp()
        }
    }

    /// Check a component
    pub fn component(ctx: &'a ir::Context, comp: &'a ir::Component) {
        Self { ctx, comp }.comp()
    }

    fn param(&self, p: ir::ParamIdx) {
        assert!(self.comp.valid(p));
    }

    /// Validate the entire component
    fn comp(&self) {
        // Validate ports
        for (pidx, _) in self.comp.ports().iter() {
            self.port(pidx);
        }

        // Validate params
        for (pidx, _) in self.comp.params().iter() {
            self.param(pidx);
        }

        // Validate invokes
        for (iidx, _) in self.comp.invocations().iter() {
            self.invoke(iidx);
        }

        // Validate instances
        for (iidx, _) in self.comp.instances().iter() {
            self.instance(iidx);
        }

        // Validate commands
        for cmd in &self.comp.cmds {
            self.command(cmd);
        }
    }

    /// A Port is valid if:
    /// (1) All bundle-owned params point to this port
    /// (2) The port's owner is defined in the component and the owner says it owns the port
    /// NOTE(rachit): A more pedantic check can enforce these in the future:
    /// (3) All time expressions are bound
    /// (4) All parameters mentioned in the range and the width are bound
    fn port(&self, pidx: ir::PortIdx) {
        let ir::Port { owner, live, .. } = self.comp.get(pidx);
        // check (1)
        if let ir::Liveness {
            idx: Some(par_idx), ..
        } = live
        {
            match self.comp.get(*par_idx).owner {
            ir::ParamOwner::Let { .. } => self.comp.internal_error(format!(
                "{} should be owned by a bundle but is owned by a let",
                self.comp.display(*par_idx)
            )),
            ir::ParamOwner::Sig => self.comp.internal_error(format!(
                "{} should be owned by a bundle but is owned by a sig",
                self.comp.display(*par_idx)
            )),
            ir::ParamOwner::Loop => self.comp.internal_error(format!(
                "{} should be owned by a bundle but is owned by a loop",
                self.comp.display(*par_idx)
            )),
            ir::ParamOwner::Exists { .. } => self.comp.internal_error(format!(
                "{} should be owned by a bundle is an existentially quantified param",
                self.comp.display(*par_idx)
            )),
            ir::ParamOwner::Instance{inst, ..} => self.comp.internal_error(format!(
                "{} should be owned by a bundle but is owned by instance {inst}",
                self.comp.display(*par_idx)
            )),
            ir::ParamOwner::Bundle(port_idx) => {
                // Ensure that the bundle-owned param points here
                if port_idx != pidx {
                    self.comp.internal_error(
                        format!("{par_idx} should be owned by {pidx} but is owned by {port_idx}"))
                }
            }
        }
        }

        // check (2)
        match owner {
            ir::PortOwner::Sig { .. } => {
                /* Can't check since the component doesn't track which ports are in the sig */
            }
            ir::PortOwner::Inv { inv: iidx, .. } => {
                let inv = &self.comp[*iidx];
                if !inv.ports.contains(&pidx) {
                    self.comp.internal_error(
                        format!("{pidx} claims to be owned by {iidx} but the invocation does not own it"))
                }
            }
            ir::PortOwner::Local => {
                if let Some((iidx, _)) = self
                    .comp
                    .invocations()
                    .iter()
                    .find(|(_, inv)| inv.ports.contains(&pidx))
                {
                    self.comp.internal_error(
                        format!("{pidx} claims to be a local port but is owned by {iidx}")
                    )
                }
            }
        }
    }

    /// An invoke is valid if:
    /// (1) Its ports are defined in the component
    /// (2) Ports defined by invoke point to it
    ///     i.  port() checks that the invoke owns the port
    ///         invoke() checks that the ports an invoke defines are owned by it
    /// (3) Its events are valid
    /// (4) Its events point to the invoke as their owner
    fn invoke(&self, iidx: ir::InvIdx) {
        assert!(self.comp.valid(iidx));

        let ir::Invoke { ports, .. } = &self.comp.get(iidx);

        // check (1) and (2)
        for pidx in ports {
            // (1) looking up the port will error if it doesn't exist
            let port = self.comp.get(*pidx);
            match port.owner {
                ir::PortOwner::Sig { .. } | ir::PortOwner::Local => {
                    self.comp.internal_error(
                        format!("{iidx} defines {pidx}, but {pidx} does not point to {iidx} as its owner")
                    );
                }
                // (2) check that each port's owner is this inv
                ir::PortOwner::Inv {
                    inv: iidx_lookup, ..
                } => {
                    if iidx != iidx_lookup {
                        self.comp.internal_error(
                            format!("{iidx} defines {pidx}, but {pidx} points to {iidx_lookup} as its owner")
                        );
                    }
                }
            }
        }
    }

    /// An instance is valid if:
    /// (1) It is defined in the component
    /// (2) Its params are defined in the component
    /// (3) The component it's instantiating is defined in the context
    /// (4) The number of params passed in matches the amount present
    ///     in the component signature
    fn instance(&self, iidx: ir::InstIdx) {
        // check (1)
        let ir::Instance {
            comp, args: params, ..
        } = &self.comp[iidx];
        // check (3) and (4)
        let comp_params = self
            .ctx
            .get(*comp)
            .params()
            .iter()
            .filter(|(_, param)| param.is_sig_owned())
            .count();
        let inst_len = params.len();
        if comp_params != inst_len {
            self.comp.internal_error(
                format!("{comp} takes {comp_params} params, but {inst_len} were passed by {iidx}")
            )
        }
    }

    fn bundle_def(&self, b: ir::PortIdx) {
        // The port in a bundle def must have a local owner
        let ir::Port { owner, .. } = &self.comp[b];
        match owner {
            ir::PortOwner::Local => {}
            _ => self.comp.internal_error(format!(
                "{b} is a bundle def, but its owner is not local"
            )),
        }
    }

    /// A command is valid if:
    /// (1) The structures that it contains are valid
    fn command(&self, cmd: &ir::Command) {
        match cmd {
            ir::Command::Instance(iidx) => self.instance(*iidx),
            ir::Command::Invoke(iidx) => self.invoke(*iidx),
            ir::Command::Connect(con) => self.connect(con),
            ir::Command::ForLoop(lp) => self.forloop(lp),
            ir::Command::If(cond) => self.if_stmt(cond),
            ir::Command::Let(l) => self.let_(l),
            ir::Command::Fact(_) => (),
            ir::Command::BundleDef(b) => self.bundle_def(*b),
            ir::Command::Exists(e) => self.exists(e),
        }
    }

    fn let_(&self, l: &ir::Let) {
        let ir::Let { param, expr } = l;
        let owner = &self.comp[*param].owner;
        let ir::ParamOwner::Let { bind } = owner else {
            self.comp.internal_error(format!(
                "{} is a let binding, but its owner is not a let",
                self.comp.display(*param)
            ))
        };
        if bind != expr {
            self.comp.internal_error(format!(
                "let binding for `{}' binds it to {} but the owner defines it to {}",
                self.comp.display(*param),
                self.comp.display(*expr),
                self.comp.display(*bind)
            ))
        }
    }

    /// A connect is valid if:
    /// (1) Both of the accesses it makes are valid
    /// NOTE(ethan): harder to check, maybe not worth it?
    /// Would have to resolve the start/end exprs, which requires a binding...
    /// (2) The range of the src and dst accesses match
    fn connect(&self, connect: &ir::Connect) {
        let ir::Connect { src, dst, .. } = connect;
        self.access(src);
        self.access(dst);
    }

    /// An access is valid if:
    /// (1) The port being accessed is valid
    /// (2) Its start and end exprs are defined in the comp
    fn access(&self, access: &ir::Access) {
        let ir::Access { port, .. } = *access;
        self.port(port);
        // self.expr(start);
        // self.expr(end);
    }

    /// A loop is valid if:
    /// (1) Its index is valid
    /// (2) Its start/end is valid
    /// (3) Everything in its body is valid
    fn forloop(&self, lp: &ir::Loop) {
        let ir::Loop { body, .. } = lp;
        // self.param(*index);
        // self.expr(*start);
        // self.expr(*end);
        for cmd in body {
            self.command(cmd);
        }
    }

    /// An if-statement is valid if:
    /// (1) Its condition is valid
    /// (2) Everything in its then-branch is valid
    /// (3) Everything in its alt-branch is valid
    fn if_stmt(&self, if_stmt: &ir::If) {
        let ir::If { then, alt, .. } = if_stmt;
        for cmd in then {
            self.command(cmd);
        }
        for cmd in alt {
            self.command(cmd);
        }
    }
    fn exists(&self, exists: &ir::Exists) {
        let ir::Exists { param: p_idx, .. } = exists;
        let param = self.comp.get(*p_idx);
        // let param = self.param(*p_idx);
        if !matches!(param.owner, ir::ParamOwner::Exists { .. }) {
            self.comp.internal_error(format!(
                "{} mentioned in existential binding but owned by {}",
                self.comp.display(*p_idx),
                param.owner
            ))
        }
        // self.expr(*expr);
    }
}
