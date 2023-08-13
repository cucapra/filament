use crate::{
    cmdline,
    ir::{Connect, Context, Ctx, DisplayCtx, PortIdx},
    ir_visitor::{Action, Construct, Visitor, VisitorData},
    utils::{GPosIdx, GlobalPositionTable},
};
use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;

/// Makes sure each index in a port is only written to at most once
/// Must occur after monomorphization.
pub struct AssignCheck {
    ports: LinkedHashMap<(PortIdx, usize), Vec<Option<GPosIdx>>>,
    diagnostic_count: u64,
}

impl Construct for AssignCheck {
    fn from(_: &cmdline::Opts, _: &mut Context) -> Self {
        Self {
            ports: LinkedHashMap::new(),
            diagnostic_count: 0,
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
        // Report all the errors
        let is_tty = atty::is(atty::Stream::Stderr);
        let writer = StandardStream::stderr(if is_tty {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
        let table = GlobalPositionTable::as_ref();

        for ((port, idx), connects) in self.ports.drain() {
            let l = connects.len();
            if l == 1 {
                continue;
            }
            let p = data.comp.get(port);
            let info = data.comp.get(p.info).as_port();

            // generate the base error message
            let diag = Diagnostic::error().with_message(format!(
                "Port {}{{{}}} {}",
                data.comp.display(port),
                idx,
                if l == 0 {
                    "is never assigned to."
                } else {
                    "is assigned to multiple times."
                }
            ));

            // add the location of the port definition
            let diag = match info {
                None => diag,
                Some(info) => diag.with_labels(vec![info
                    .bind_loc
                    .secondary()
                    .with_message("defined here")]),
            };

            // filter the assignments that have bind location information
            let filtered_cons = connects.into_iter().flatten().collect_vec();
            // count the number of assignments that don't have bind location information
            let missing_info = l - filtered_cons.len();

            // add the locations of the assignments
            let diag = diag.with_labels(
                filtered_cons
                    .into_iter()
                    .map(|idx| idx.primary().with_message("assigned here"))
                    .collect(),
            );

            // add a note about the number of assignments that don't have bind location information
            let diag = match missing_info {
                0 => diag,
                1 => diag.with_notes(vec![
                    "Also assigned in 1 other location".to_string()
                ]),
                _ => diag.with_notes(vec![format!(
                    "Also assigned in {} other locations",
                    missing_info
                )]),
            };

            term::emit(
                &mut writer.lock(),
                &term::Config::default(),
                table.files(),
                &diag,
            )
            .unwrap();

            self.diagnostic_count += 1;
        }
    }

    fn after_traversal(&mut self) -> Option<u64> {
        if self.diagnostic_count > 0 {
            Some(self.diagnostic_count)
        } else {
            None
        }
    }
}
