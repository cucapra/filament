use crate::{ir, ir::Ctx};

use super::IndexStore;
/// Implements validation checks for IR data structures.
/// If calling the methods in this does not result in a panic, then the corresponding IR structure is valid.
/// The validity condition for each structure is defined in the corresponding method.
pub struct Validate<'a> {
    /// The component being validated.
    comp: &'a ir::Component,
    /// The context for the program being evaluated
    ctx: &'a IndexStore<ir::Component>,
}

impl<'a> Validate<'a> {
    /// Validate the entire component
    pub fn comp(&self) {
        // Validate exprs
        for (eidx, _) in self.comp.exprs().iter() {
            self.expr(eidx);
        }

        // Validate times
        for (tidx, _) in self.comp.times().iter() {
            self.time(tidx);
        }

        // Validate props
        for (pidx, _) in self.comp.props().iter() {
            self.prop(pidx);
        }

        // Validate ports
        for (pidx, _) in self.comp.ports().iter() {
            self.port(pidx);
        }

        // Validate params
        for (pidx, _) in self.comp.params().iter() {
            self.param(pidx);
        }

        // Validate events
        for (evidx, _) in self.comp.events().iter() {
            self.event(evidx);
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

    /// An Expr is valid if:
    /// (1) It is bound in the component
    pub fn expr(&self, eidx: ir::ExprIdx) {
        let _ = &self.comp[eidx];
    }

    /// A Port is valid if:
    /// (1) All bundle-owned params point to this port
    /// (2) The port's owner is defined in the component and the owner says it owns the port
    /// NOTE(rachit): A more pedantic check can enforce these in the future:
    /// (3) All time expressions are bound
    /// (4) All parameters mentioned in the range and the width are bound
    pub fn port(&self, pidx: ir::PortIdx) {
        let ir::Port {
            owner, width, live, ..
        } = self.comp.get(pidx);
        // check (1)
        let ir::Liveness {
            idx: par_idx,
            len,
            range,
        } = live;
        match self.comp.get(*par_idx).owner {
            ir::ParamOwner::Sig => self.comp.internal_error(format!(
                "{par_idx} should be owned by a bundle but is owned by a sig"
            )),
            ir::ParamOwner::Loop => self.comp.internal_error(format!(
                "{par_idx} should be owned by a bundle but is owned by a loop"
            )),
            ir::ParamOwner::Bundle(port_idx) => {
                // Ensure that the bundle-owned param points here
                if port_idx != pidx {
                    self.comp.internal_error(
                        format!("{par_idx} should be owned by {pidx} but is owned by {port_idx}"))
                }
            }
        }
        // validate liveness length
        self.expr(*len);

        // check range by checking both the Times it uses
        let ir::Range { start, end } = range;
        self.time(*start);
        self.time(*end);

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
        self.expr(*width);
    }

    /// An event is valid if:
    /// (1) Its owner is defined in the component and says it owns the event
    /// (2) Its delay is valid
    pub fn event(&self, evidx: ir::EventIdx) {
        let ir::Event { delay, .. } = &self.comp[evidx];

        // check (2)
        self.timesub(delay);
    }

    /// A TimeSub is valid if:
    /// (1) Its fields are all well-formed, i.e.
    ///     i. If it is a Unit, its expr exists in the component
    ///     ii. If it is a Sym, both of its times are well-formed
    pub fn timesub(&self, ts: &ir::TimeSub) {
        // check (1)
        match ts {
            ir::TimeSub::Unit(expr) => {
                self.expr(*expr);
            }
            ir::TimeSub::Sym {
                l: t1_idx,
                r: t2_idx,
            } => {
                self.time(*t1_idx);
                self.time(*t2_idx);
            }
        }
    }

    /// A Time is valid if:
    /// (1) It is defined in the component
    /// (2) Its fields are defined in the component
    pub fn time(&self, tidx: ir::TimeIdx) {
        // check (1)
        let ir::Time { event, offset } = &self.comp[tidx];

        // check (2)
        self.event(*event);
        self.expr(*offset);
    }

    /// A Range is valid if:
    /// (1) Both its start and end times are valid
    pub fn range(&self, range: ir::Range) {
        let ir::Range { start, end } = range;
        self.time(start);
        self.time(end);
    }

    /// A param is valid if:
    /// (1) It is defined in the component
    /// (2) Its owner is defined in the component
    /// (3) Its owner points to it?
    pub fn param(&self, pidx: ir::ParamIdx) {
        // check (1) - this will panic if param not defined
        let ir::Param { owner, .. } = &self.comp.get(pidx);

        // check (2) and (3)
        match owner {
            ir::ParamOwner::Sig | ir::ParamOwner::Loop => {
                /* Nothing to check */
            }
            ir::ParamOwner::Bundle(port_idx) => {
                let ir::Port { live, .. } = &self.comp.get(*port_idx); // (2) this will panic if port not defined

                // check (3)
                let ir::Liveness { idx, .. } = live;
                if *idx != pidx {
                    self.comp.internal_error(
                        format!("{pidx} points to {port_idx} as its owner, but {port_idx} uses {idx}")
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
        let ir::Instance { comp, params } = &self.comp[iidx];
        for expr in params.iter() {
            // check (2)
            self.expr(*expr);
        }
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

    /// A command is valid if:
    /// (1) The structures that it contains are valid
    fn command(&self, cmd: &ir::Command) {
        match cmd {
            ir::Command::Instance(iidx) => {
                self.instance(*iidx);
            }
            ir::Command::Invoke(iidx) => {
                self.invoke(*iidx);
            }
            ir::Command::Connect(con) => {
                self.connect(con);
            }
            ir::Command::ForLoop(lp) => {
                self.forloop(lp);
            }
            ir::Command::If(cond) => {
                self.if_stmt(cond);
            }
            ir::Command::Fact(fact) => {
                self.fact(fact);
            }
        }
    }

    /// A prop is valid if:
    /// (1) It is defined in the component
    /// (2) The structures its made of are valid
    fn prop(&self, pidx: ir::PropIdx) {
        let prop = &self.comp[pidx];
        match prop {
            ir::Prop::True | ir::Prop::False => { /* Nothing to do */ }
            ir::Prop::Cmp(cmp) => {
                let ir::CmpOp { op: _, lhs, rhs } = cmp;
                self.expr(*lhs);
                self.expr(*rhs);
            }
            ir::Prop::TimeCmp(tcmp) => {
                let ir::CmpOp { op: _, lhs, rhs } = tcmp;
                self.time(*lhs);
                self.time(*rhs);
            }
            ir::Prop::TimeSubCmp(tscmp) => {
                let ir::CmpOp { op: _, lhs, rhs } = tscmp;
                self.timesub(lhs);
                self.timesub(rhs);
            }
            ir::Prop::Not(pidx) => {
                self.prop(*pidx);
            }
            ir::Prop::And(pidx1, pidx2) => {
                self.prop(*pidx1);
                self.prop(*pidx2);
            }
            ir::Prop::Or(pidx1, pidx2) => {
                self.prop(*pidx1);
                self.prop(*pidx2);
            }
            ir::Prop::Implies(pidx1, pidx2) => {
                self.prop(*pidx1);
                self.prop(*pidx2);
            }
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
        let ir::Access { port, start, end } = *access;
        self.port(port);
        self.expr(start);
        self.expr(end);
    }

    /// A loop is valid if:
    /// (1) Its index is valid
    /// (2) Its start/end is valid
    /// (3) Everything in its body is valid
    fn forloop(&self, lp: &ir::Loop) {
        let ir::Loop {
            index,
            start,
            end,
            body,
        } = lp;
        self.param(*index);
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
        self.prop(*cond);
        for cmd in then {
            self.command(cmd);
        }
        for cmd in alt {
            self.command(cmd);
        }
    }

    /// A fact is valid if:
    /// (1) Its prop is valid
    fn fact(&self, fact: &ir::Fact) {
        let ir::Fact { prop, .. } = *fact;
        self.prop(prop);
    }
}
