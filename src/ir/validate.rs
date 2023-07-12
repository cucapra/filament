use crate::{ir, ir::Ctx};
/// Implements validation checks for IR data structures.
/// If calling the methods in this does not result in a panic, then the corresponding IR structure is valid.
/// The validity condition for each structure is defined in the corresponding method.
pub struct Validate<'a> {
    /// The component being validated.
    comp: &'a ir::Component,
}

impl<'a> Validate<'a> {
    /// Validate the entire component
    pub fn comp(&self) {

        // Validate ports
        for (pidx, _) in self.comp.ports().iter() {
            self.port(pidx);
        }
    }

    /// A port is valid if:
    /// (1) All bundle-owned params point to this port
    /// (2) The port's owner is defined in the component and the owner says it owns the port
    /// NOTE(rachit): A more pedantic check can enforce these in the future:
    /// (3) All time expressions are bound
    /// (4) All parameters mentioned in the range and the width are bound
    pub fn port(&self, pidx: ir::PortIdx) {
        let ir::Port { owner, live, .. } = self.comp.get(pidx);
        // check (1)
        let ir::Liveness {idx: par_idx, ..} = live;
        match self.comp.get(*par_idx).owner {
            ir::ParamOwner::Sig => {
                self.comp.internal_error(
                    format!("{par_idx} should be owned by a bundle but is owned by a sig")
                )
            }
            ir::ParamOwner::Loop => {
                self.comp.internal_error(
                    format!("{par_idx} should be owned by a bundle but is owned by a loop")
                )
            }
            ir::ParamOwner::Bundle(port_idx) => {
                // Ensure that the bundle-owned param points here
                if port_idx != pidx {
                    self.comp.internal_error(
                        format!("{par_idx} should be owned by {pidx} but is owned by {port_idx}"))
                }
            }
        }
        // check len?

        // check range

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

        // let p: for<#i> [G+#N, G+i+1]
        // let p: for<%pr1> [G+%pr2, G+%pr1+1]
        // let p0: for<%pr3> [G+%pr2, G+%pr1+1] %pr1 -> %pr3
        // %pr1: owned by port p
    }

    /// An event is valid if:
    /// (1) Its owner is defined in the component and says it owns the event
    /// (2) Its delay is valid
    pub fn event(&self, evidx: ir::EventIdx) {
        let ir::Event {delay, owner, ..} = self.comp.get(evidx);

        // check (1)
        match owner {
            ir::EventOwner::Sig => {
                /* Can't check because the sig does not contain this info */
            }
            ir::EventOwner::Inv {inv: iidx} => {
                let inv = &self.comp[*iidx];
                // TODO: merge in changes where Invokes track events
            }
        }
    }

    /// A param is valid if:
    /// (1) It is defined in the component
    /// (2) Its owner is defined in the component
    /// (3?) Its owner points to it?
    pub fn param(&self, pidx: ir::ParamIdx) {
        // check (1) - this will panic if param not defined
        let ir::Param {owner, ..} = &self.comp.get(pidx);

        // check (2)
        match owner {
            ir::ParamOwner::Sig | ir::ParamOwner::Loop => {
                /* Nothing to check */
            }
            ir::ParamOwner::Bundle(port_idx) => {
                let ir::Port {
                    owner,
                    width,
                    live,
                    info
                } = &self.comp.get(*port_idx); // this will panic if portt not defined
            }
        }
    }

    /// An invoke is valid if:
    /// (1) Its ports are defined in the component
    /// (2) Ports defined by invoke point to it
    ///     i.  port() checks that the invoke owns the port
    ///         invoke() checks that the ports an invoke defines are owned by it
    fn invoke(&self, iidx: ir::InvIdx) {
        let ir::Invoke {inst, ports} = &self.comp.get(iidx);

        // check (1) and (2)
        for pidx in ports {
            // (1) looking up the port will error if it doesn't exist
            let ir::Port {owner, ..} = &self.comp.get(*pidx);
            match owner {
                ir::PortOwner::Sig {..} | ir::PortOwner::Local => {
                    self.comp.internal_error(
                        format!("{iidx} defines {pidx}, but {pidx} does not point to {iidx} as its owner")
                    );
                }
                ir::PortOwner::Inv {inv: iidx_lookup, ..} => {
                    if iidx != *iidx_lookup {
                        self.comp.internal_error(
                            format!("{iidx} defines {pidx}, but {pidx} points to {iidx_lookup} as its owner")
                        );
                    }
                }
            }
        }
    }
}
