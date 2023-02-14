use crate::{
    errors::Error,
    utils::{GPosIdx, GlobalPositionTable},
};
use codespan_reporting::term::termcolor::ColorChoice;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle},
    term::{self, termcolor::StandardStream},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Index for information associated with a [Diagnostic] instance.
// XXX: Define `add_message` and `add_info` on this type so that user code can use it as a builder pattern.
pub struct InfoIdx(usize);

/// Information attached to a [Diagnostic] instance.
// XXX: Change this to track a vector of messages and positions which logically
// correspond to one error.
struct Information {
    message: String,
    pos: GPosIdx,
}

impl Information {
    fn new(message: String, pos: GPosIdx) -> Self {
        Self { message, pos }
    }
}

#[derive(Default)]
/// Track diagnostics that need to be reported
pub struct Diagnostics {
    /// Information being tracked by this instance.
    infos: Vec<Information>,
    /// Errors that have been reported.
    errors: Vec<Error>,
}

impl Diagnostics {
    /// Add information to the diagnostics instance.
    pub fn add_info<S: ToString>(
        &mut self,
        message: S,
        pos: GPosIdx,
    ) -> InfoIdx {
        let idx = self.infos.len();
        self.infos.push(Information::new(message.to_string(), pos));
        InfoIdx(idx)
    }

    /// Add a message without any position information.
    pub fn add_message<S: ToString>(&mut self, message: S) -> InfoIdx {
        self.add_info(message.to_string(), GPosIdx::UNKNOWN)
    }

    /// Add an error to the diagnostics instance.
    // XXX: Make this add a new information object so that its easy to express
    // the "create error and add info" pattern.
    pub fn add_error(&mut self, error: Error) {
        self.errors.push(error);
    }

    /// Report all errors and return the number of errors.
    /// Returns None if there are no errors.
    pub fn report_all(self) -> Option<u64> {
        // XXX: Pass no_color opt
        let writer = StandardStream::stderr(if false {
            ColorChoice::Never
        } else {
            ColorChoice::Always
        });
        let num_errors = self.errors.len();

        let table = GlobalPositionTable::as_ref();
        for err in self.errors {
            let mut labels = vec![];
            let mut notes = vec![];
            for (idx, info) in err.notes.iter().enumerate() {
                let info = &self.infos[info.0];
                if let Some(p) = info.pos.into_option() {
                    let pos = table.get_pos(p.0);
                    let l = Label::new(
                        if idx == 0 {
                            LabelStyle::Primary
                        } else {
                            LabelStyle::Secondary
                        },
                        pos.file.get(),
                        pos.start..pos.end,
                    );
                    labels.push(l.with_message(info.message.clone()));
                } else {
                    notes.push(info.message.clone());
                }
            }

            term::emit(
                &mut writer.lock(),
                &term::Config::default(),
                table.files(),
                &Diagnostic::error()
                    .with_message(format!("{}", err.kind))
                    .with_labels(labels)
                    .with_notes(notes),
            )
            .unwrap();
        }

        Some(num_errors as u64)
    }

    /// Signals an unrecoverable error.
    /// Prints the current diagnostics and exits the program.
    pub fn unrecoverable(&mut self) -> ! {
        todo!()
    }
}
