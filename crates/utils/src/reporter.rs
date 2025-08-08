use crate::{Error, GPosIdx, GlobalPositionTable, Severity};
use codespan_reporting::term::termcolor::ColorChoice;
use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle},
    term::{self, termcolor::StandardStream},
};
use std::collections::BTreeMap;

#[derive(PartialOrd, Ord, Clone, Copy, Debug, PartialEq, Eq, Hash)]
/// Index for information associated with a [Diagnostic] instance.
// XXX: Define `add_message` and `add_info` on this type so that user code can use it as a builder pattern.
pub struct InfoIdx(usize);

#[derive(Eq, PartialEq)]
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
    /// Warnings that have been reported.
    warnings: Vec<Error>,
}

impl Diagnostics {
    fn add_info_help(&mut self, info: Information) -> InfoIdx {
        if let Some(idx) = self.infos.iter().position(|i| *i == info) {
            InfoIdx(idx)
        } else {
            let idx = self.infos.len();
            self.infos.push(info);
            InfoIdx(idx)
        }
    }

    /// Add information to the diagnostics instance.
    pub fn add_info<S: ToString>(
        &mut self,
        message: S,
        pos: GPosIdx,
    ) -> InfoIdx {
        if pos == GPosIdx::UNKNOWN {
            log::trace!(
                "Adding info with unknown position: {}",
                message.to_string()
            )
        }
        self.add_info_help(Information::new(message.to_string(), pos))
    }

    /// Add a message without any position information.
    pub fn add_message<S: ToString>(&mut self, message: S) -> InfoIdx {
        self.add_info_help(Information::new(
            message.to_string(),
            GPosIdx::UNKNOWN,
        ))
    }

    /// Add an error to the diagnostics instance.
    // XXX: Make this add a new information object so that its easy to express
    // the "create error and add info" pattern.
    pub fn add_error(&mut self, error: Error) {
        match error.severity {
            Severity::Error => {
                if !self.errors.contains(&error) {
                    log::trace!("Adding error: {}", error.kind);
                    self.errors.push(error);
                }
            }
            Severity::Warning => {
                if !self.warnings.contains(&error) {
                    log::trace!("Adding warning: {}", error.kind);
                    self.warnings.push(error);
                }
            }
        }
    }

    /// Add a warning to the diagnostics instance.
    pub fn add_warning(&mut self, warning: Error) {
        let warning = Error { severity: Severity::Warning, ..warning };
        self.add_error(warning);
    }

    /// Report all errors and warnings. Returns Some(error_count) if there are errors.
    /// Warnings do not count toward compilation failure.
    pub fn report_all(&mut self) -> Option<u64> {
        let is_tty = atty::is(atty::Stream::Stderr);
        let writer = StandardStream::stderr(if is_tty {
            ColorChoice::Always
        } else {
            ColorChoice::Never
        });
        
        // Take ownership to avoid borrowing issues
        let warnings = std::mem::take(&mut self.warnings);
        let errors = std::mem::take(&mut self.errors);
        
        // Report warnings first
        self.report_diagnostics_owned(&writer, warnings, Severity::Warning);
        
        // Report errors and return count
        let error_count = self.report_diagnostics_owned(&writer, errors, Severity::Error);

        if error_count > 0 {
            Some(error_count)
        } else {
            None
        }
    }

    fn report_diagnostics_owned(&self, writer: &StandardStream, mut diagnostics: Vec<Error>, severity: Severity) -> u64 {
        if diagnostics.is_empty() {
            return 0;
        }

        let mut total = 0;

        // Deduplicate diagnostics based on the location attached to the diagnostic
        let mut diagnostic_map = BTreeMap::new();
        for mut diagnostic in diagnostics.drain(..) {
            if !diagnostic.notes.is_empty() {
                // Sort everything except the first element
                let first = diagnostic.notes.remove(0);
                diagnostic.notes.sort();
                diagnostic.notes.insert(0, first);
            }

            diagnostic_map
                .entry(diagnostic.notes)
                .or_insert_with(Vec::new)
                .push(diagnostic.kind);
        }

        let table = GlobalPositionTable::get();
        for (all_notes, messages) in diagnostic_map {
            let mut labels = vec![];
            let mut notes = vec![];
            for (idx, info) in all_notes.iter().enumerate() {
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

            let msg = if messages.len() > 1 {
                notes.extend(messages.iter().map(|e| e.to_string()));
                match severity {
                    Severity::Error => "Multiple errors encountered".to_string(),
                    Severity::Warning => "Multiple warnings encountered".to_string(),
                }
            } else {
                messages[0].to_string()
            };

            total += 1;
            let diagnostic_type = match severity {
                Severity::Error => Diagnostic::error(),
                Severity::Warning => Diagnostic::warning(),
            };
            
            term::emit(
                &mut writer.lock(),
                &term::Config::default(),
                table.files(),
                &diagnostic_type
                    .with_message(msg)
                    .with_labels(labels)
                    .with_notes(notes),
            )
            .unwrap();
        }

        total
    }

    /// Check if there are any errors (not warnings)
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }
}

impl Drop for Diagnostics {
    fn drop(&mut self) {
        if let Some(errs) = self.report_all() {
            if std::thread::panicking() {
                eprintln!("{errs} errors encountered before panic");
            }
        }
    }
}
