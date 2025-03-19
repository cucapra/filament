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
        for pidx in self.comp.ports().idx_iter() {
            self.port(pidx);
        }

        // Validate params
        for pidx in self.comp.params().idx_iter() {
            self.param(pidx);
        }

        // Validate invokes
        for iidx in self.comp.invocations().idx_iter() {
            self.invoke(iidx);
        }

        // Validate instances
        for iidx in self.comp.instances().idx_iter() {
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
    /// (3) All time expressions are bound
    /// (4) All parameters mentioned in the range and the width are bound
    fn port(&self, pidx: ir::PortIdx) {
        let ir::Port {
            owner, live, width, ..
        } = self.comp.get(pidx);
        // check (1)
        let ir::Liveness {
            idxs: par_idxs,
            lens,
            range,
        } = live;

        for par_idx in par_idxs {
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

        // check (3)
        self.range(range);

        // check (4)
        self.expr(*width);
        for len in lens {
            self.expr(*len);
        }
    }

    /// An invoke is valid if:
    /// (1) Its ports are defined in the component
    /// (2) Ports defined by invoke point to it
    ///     i.  port() checks that the invoke owns the port
    ///         invoke() checks that the ports an invoke defines are owned by it
    /// (3) Its event bindings are valid
    fn invoke(&self, iidx: ir::InvIdx) {
        assert!(self.comp.valid(iidx));

        let ir::Invoke { ports, events, .. } = &self.comp.get(iidx);

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
        // check (3)
        for ir::EventBind {
            arg, delay, base, ..
        } in events
        {
            self.time(*arg);
            self.timesub(delay);
            // Validate that the foreign event exists
            assert!(base.apply(|e, c| c.valid(e), self.ctx));
        }
    }

    /// An instance is valid if:
    /// (1) It is defined in the component
    /// (2) Its existential params are defined in the component
    /// (3) Its ranges are valid
    /// (4) The component it's instantiating is defined in the context
    /// (5) The number of params passed in matches the amount present
    ///     in the component signature
    fn instance(&self, iidx: ir::InstIdx) {
        // check (1)
        let ir::Instance {
            comp,
            args,
            params,
            lives,
            ..
        } = &self.comp[iidx];
        // check (2)
        for param in params {
            self.param(*param);
        }
        // check (3)
        for live in lives {
            self.range(live);
        }
        // check (4) and (5)
        let comp_params = self
            .ctx
            .get(*comp)
            .params()
            .iter()
            .filter(|(_, param)| param.is_sig_owned())
            .count();
        let inst_len = args.len();
        if comp_params != inst_len {
            self.comp.internal_error(
                format!("{comp} takes {comp_params} params, but {inst_len} were passed by {iidx}")
            )
        }
    }

    fn bundle_def(&self, b: ir::PortIdx) {
        self.port(b);
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
            ir::Command::Fact(f) => self.fact(f),
            ir::Command::BundleDef(b) => self.bundle_def(*b),
            ir::Command::Exists(e) => self.exists(e),
        }
    }

    fn fact(&self, fact: &ir::Fact) {
        self.prop(fact.prop);
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
                match expr {
                    Some(expr) => self.comp.display(*expr),
                    None => "?".to_string(),
                },
                match bind {
                    Some(bind) => self.comp.display(*bind),
                    None => "?".to_string(),
                }
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
    /// (2) The start and end exprs of its ranges are defined in the comp
    fn access(&self, access: &ir::Access) {
        let ir::Access { port, ranges } = access;
        self.port(*port);
        for (start, end) in ranges {
            self.expr(*start);
            self.expr(*end);
        }
    }

    /// A loop is valid if:
    /// (1) Its index is valid and owned by a loop
    /// (2) Its start/end is valid
    /// (3) Everything in its body is valid
    fn forloop(&self, lp: &ir::Loop) {
        let ir::Loop {
            body,
            start,
            end,
            index,
        } = lp;
        let param = self.comp.get(*index);
        if !matches!(param.owner, ir::ParamOwner::Loop) {
            self.comp.internal_error(format!(
                "{} mentioned in loop but owned by {}",
                self.comp.display(*index),
                param.owner
            ))
        }
        self.expr(*start);
        self.expr(*end);
        for cmd in body {
            self.command(cmd);
        }
    }

    /// An if-statement is valid if:
    /// (1) Its condition is valid
    /// (2) Everything in its then-branch is valid
    /// (3) Everything in its alt-branch is valid
    fn if_stmt(&self, if_stmt: &ir::If) {
        let ir::If { cond, then, alt } = if_stmt;
        // check (1)
        self.prop(*cond);
        // check (2)
        for cmd in then {
            self.command(cmd);
        }
        // check (3)
        for cmd in alt {
            self.command(cmd);
        }
    }
    fn exists(&self, exists: &ir::Exists) {
        let ir::Exists { param: p_idx, expr } = exists;
        let param = self.comp.get(*p_idx);
        // let param = self.param(*p_idx);
        if !matches!(param.owner, ir::ParamOwner::Exists { .. }) {
            self.comp.internal_error(format!(
                "{} mentioned in existential binding but owned by {}",
                self.comp.display(*p_idx),
                param.owner
            ))
        }
        self.expr(*expr);
    }

    /// A prop is valid iff all of its fields are valid
    fn prop(&self, pidx: ir::PropIdx) {
        assert!(pidx.valid(self.comp));
    }

    /// An expr is valid iff all of its arguments are valid
    fn expr(&self, eidx: ir::ExprIdx) {
        assert!(eidx.valid(self.comp));
    }

    /// A time is valid iff all of its arguments are valid
    fn time(&self, tidx: ir::TimeIdx) {
        assert!(tidx.valid(self.comp));
    }

    fn timesub(&self, ts: &ir::TimeSub) {
        match ts {
            crate::TimeSub::Unit(idx) => self.expr(*idx),
            crate::TimeSub::Sym { l, r } => {
                self.time(*l);
                self.time(*r);
            }
        }
    }

    fn range(&self, r: &ir::Range) {
        self.time(r.start);
        self.time(r.end);
    }
}
