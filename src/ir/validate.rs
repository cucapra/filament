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
    pub fn comp() {
        todo!()
    }

    /// A port is valid if:
    /// - All bundle-owned params point to this port
    /// - The port's owner is defined in the component and the owner says it owns the port
    /// NOTE(rachit): A more pedantic check can enforce these in the future:
    /// - All time expressions are bound
    /// - All parameters mentioned in the range and the width are bound
    pub fn port(&self, pidx: ir::PortIdx) {
        let ir::Port { owner, live, .. } = self.comp.get(pidx);
        match owner {
            ir::PortOwner::Sig { .. } => {
                /* Can't check since the component doesn't track which ports are in the sig */
            }
            ir::PortOwner::Inv { inv: iidx, .. } => {
                let inv = &self.comp[*iidx];
                if !inv.ports.contains(&pidx) {
                    self.comp.internal_error(format!("{pidx} claims to be owned by {iidx} but the invocation does not own it"))
                }
            }
            ir::PortOwner::Local => {
                if let Some((iidx, _)) = self
                    .comp
                    .invocations()
                    .iter()
                    .find(|(_, inv)| inv.ports.contains(&pidx))
                {
                    self.comp.internal_error(format!("{pidx} claims to be a local port but is owned by {iidx}"))
                }
            }
        }

        // let p: for<#i> [G+#N, G+i+1]
        // let p: for<%pr1> [G+%pr2, G+%pr1+1]
        // let p0: for<%pr3> [G+%pr2, G+%pr1+1] %pr1 -> %pr3
        // %pr1: owned by port p
    }
}
