use crate::{
    cmdline,
    ir_visitor::{Action, Construct, Visitor, VisitorData},
};
use fil_ir::{Connect, Context, Ctx, DisplayCtx, PortIdx};
use fil_utils::{Diagnostics, Error, GPosIdx};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;

/// Makes sure each index in a port is only written to at most once
/// Must occur after monomorphization.
pub struct AssignCheck {
    ports: LinkedHashMap<(PortIdx, usize), Vec<Option<GPosIdx>>>,
    diag: Diagnostics,
}

impl Construct for AssignCheck {
    fn from(_: &cmdline::Opts, _: &mut Context) -> Self {
        Self {
            ports: LinkedHashMap::new(),
            diag: Diagnostics::default(),
        }
    }

    fn clear_data(&mut self) {
        self.ports = LinkedHashMap::new();
    }
}

impl Visitor for AssignCheck {
    fn name() -> &'static str {
        "assign-check"
    }

    fn start(&mut self, data: &mut VisitorData) -> Action {
        // skip externals
        if data.comp.is_ext {
            return Action::Stop;
        }

        for (idx, port) in data.comp.ports().iter() {
            // input ports and invoke output ports are the only ports that don't have to be written to
            if port.is_sig_in() || port.is_inv_out() {
                continue;
            }

            let len = port.live.len.concrete(&data.comp) as usize;

            for i in 0..len {
                self.ports.insert((idx, i), Vec::new());
            }
        }

        Action::Continue
    }

    fn connect(&mut self, con: &mut Connect, data: &mut VisitorData) -> Action {
        let Connect { dst, info, .. } = con;

        let start = dst.start.concrete(&data.comp) as usize;
        let end = dst.end.concrete(&data.comp) as usize;

        for i in start..end {
            self.ports
                .entry((dst.port, i))
                .or_default()
                .push(data.comp.get(*info).as_connect().map(|c| c.dst_loc));
        }
        Action::Continue
    }

    fn end(&mut self, data: &mut VisitorData) {
        let diag = &mut self.diag;
        // Track all the port locations that have no assignment
        let mut unassigned: LinkedHashMap<PortIdx, Vec<usize>> =
            LinkedHashMap::new();
        for ((port, idx), connects) in self.ports.drain() {
            let con_len = connects.len();
            // If there is exactly one assignment to this location, then it is valid
            if con_len == 1 {
                continue;
            }
            if con_len == 0 {
                unassigned.entry(port).or_default().push(idx);
                continue;
            }

            // generate the base error message
            let err = Error::malformed(format!(
                "port `{}{{{}}}' is assigned to {con_len} times",
                data.comp.display(port),
                idx,
            ));

            // Add all assignments with location information
            let err = connects.into_iter().flatten().fold(err, |err, pos| {
                err.add_note(diag.add_info("assigned here", pos))
            });
            diag.add_error(err)
        }

        for (port, mut idxs) in unassigned {
            idxs.sort();
            let err = Error::malformed(format!(
                "bundle `{}' has {} unassigned locations",
                idxs.len(),
                data.comp.display(port)
            ));

            let p = data.comp.get(port);
            let info = data.comp.get(p.info).as_port();
            // If there are more than 5 unassigned locations, then we truncate the error message
            if let Some(info) = info {
                let mut msg = format!(
                    "bundle indices are unassigned: {}",
                    idxs.iter().take(5).map(|i| i.to_string()).join(", ")
                );
                if idxs.len() > 5 {
                    msg.push_str(
                        format!(", ... and {} others", idxs.len() - 5).as_str(),
                    );
                }
                let err = err.add_note(diag.add_info(msg, info.bind_loc));
                diag.add_error(err)
            }
        }
    }

    fn after_traversal(&mut self) -> Option<u64> {
        self.diag.report_all()
    }
}
