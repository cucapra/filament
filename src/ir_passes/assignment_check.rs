use std::collections::HashMap;

use codespan_reporting::{
    diagnostic::Diagnostic,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
    },
};
use itertools::Itertools;

use crate::{
    ir::{Component, Connect, Ctx, DisplayCtx, PortIdx},
    ir_visitor::{Action, Visitor},
    utils::{GPosIdx, GlobalPositionTable},
};

#[derive(Default)]
/// Makes sure each index in a port is only written to at most once
/// Must occur after monomorphization.
pub struct AssignCheck {
    ports: HashMap<(PortIdx, usize), Vec<Option<GPosIdx>>>,
    diagnostics: Vec<Diagnostic<usize>>,
}

impl Visitor for AssignCheck {
    fn connect(&mut self, con: &mut Connect, comp: &mut Component) -> Action {
        let Connect { dst, info, .. } = con;

        let start = dst.start.as_concrete(comp).unwrap() as usize;
        let end = dst.end.as_concrete(comp).unwrap() as usize;

        for i in start..end {
            self.ports
                .entry((dst.port, i))
                .or_default()
                .push(comp.get(*info).as_connect().map(|c| c.dst_loc));
        }
        Action::Continue
    }

    fn end(&mut self, comp: &mut Component) {
        for ((port, idx), connects) in self.ports.drain() {
            let l = connects.len();
            // assigned only once, no problems
            if l <= 1 {
                continue;
            }

            let filtered_cons = connects.into_iter().flatten().collect_vec();

            let missing_info = l - filtered_cons.len();

            let diag = Diagnostic::error()
                .with_message(format!(
                    "Port {}{{{}}} assigned multiple times",
                    comp.display(port),
                    idx
                ))
                .with_labels(
                    filtered_cons
                        .into_iter()
                        .map(|idx| {
                            idx.secondary().with_message("assigned here")
                        })
                        .collect(),
                );

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

            self.diagnostics.push(diag);
        }
    }

    fn after_traversal(&mut self) -> Option<u32> {
        let err_count = self.diagnostics.len() as u32;
        // Report all the errors
        let is_tty = atty::is(atty::Stream::Stderr);
        let writer = StandardStream::stderr(if is_tty {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
        let table = GlobalPositionTable::as_ref();
        for diag in &self.diagnostics {
            term::emit(
                &mut writer.lock(),
                &term::Config::default(),
                table.files(),
                diag,
            )
            .unwrap();
        }

        if err_count > 0 {
            Some(err_count)
        } else {
            None
        }
    }
}
