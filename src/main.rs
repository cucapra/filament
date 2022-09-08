use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::Instant,
};

use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle},
    files::SimpleFiles,
    term::{self, termcolor::ColorChoice, termcolor::StandardStream},
};
use filament::{
    backend, cmdline, dump_interface, errors, event_checker, frontend,
    interval_checking, lower, visitor::Transform,
};

// Prints out the interface for main component in the input program.
fn run(opts: &cmdline::Opts) -> errors::FilamentResult<()> {
    // enable tracing
    env_logger::Builder::new()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(opts.log_level)
        .target(env_logger::Target::Stderr)
        .init();

    let mut ns = frontend::FilamentParser::parse_file(&opts.input)?;
    let mut imports: Vec<PathBuf> = ns
        .imports
        .drain(..)
        .map(|s| {
            let mut base = opts.library.clone();
            base.push(s);
            base
        })
        .collect();

    // Get the absolute paths for all externs
    let absolute = |ext_path: String, file: &Path| -> String {
        let ext: PathBuf = ext_path.into();
        if ext.is_relative() {
            let mut base: PathBuf = file
                .parent()
                .unwrap_or_else(|| panic!("Parsed file does not have a parent"))
                .into();
            base.push(ext);
            base
        } else {
            ext
        }
        .to_string_lossy()
        .to_string()
    };
    ns.externs = ns
        .externs
        .drain(..)
        .map(|(p, imps)| (absolute(p, &opts.input), imps))
        .collect();

    // Parse all the imports
    while let Some(path) = imports.pop() {
        let mut imp = frontend::FilamentParser::parse_file(&path)?;
        imp.components.append(&mut ns.components);
        ns.components = imp.components;
        ns.externs.extend(
            imp.externs
                .into_iter()
                .map(|(p, imps)| (absolute(p, &path), imps)),
        );
        imports.extend(imp.imports.into_iter().map(|s| {
            let mut base = opts.library.clone();
            base.push(s);
            base
        }));
    }

    // Run the compilation pipeline
    let t = Instant::now();
    let ns = event_checker::check_and_transform(ns)?;
    log::info!("Event check: {}ms", t.elapsed().as_millis());
    log::trace!("{ns}");
    let t = Instant::now();
    let mut ns = interval_checking::check(ns)?;
    log::info!("Interval check: {}ms", t.elapsed().as_millis());

    if opts.dump_interface || !opts.check {
        let t = Instant::now();
        ns = lower::CompileInvokes::transform(ns)?;
        log::info!("Lowering: {}ms", t.elapsed().as_millis());
        log::trace!("{ns}");
    }

    if opts.dump_interface {
        dump_interface::DumpInterface::transform(ns)?;
    } else if !opts.check {
        let t = Instant::now();
        backend::compile(ns, opts)?;
        log::info!("Compilation: {}ms", t.elapsed().as_millis());
    }

    Ok(())
}

fn main() {
    let opts: cmdline::Opts = argh::from_env();
    match run(&opts) {
        Ok(_) => (),
        Err(err) => {
            let config = term::Config::default();

            // Construct a file Map
            let mut file_map = HashMap::new();
            let mut files = SimpleFiles::new();
            for (name, file) in err.files() {
                let idx = files.add(name.clone(), file);
                file_map.insert(name, idx);
            }

            // Construct mapping from files to indices
            let mut labels = vec![];
            let mut notes = vec![];
            for (idx, (msg, pos)) in err.notes.iter().enumerate() {
                if let Some(pos) = pos {
                    let l = Label::new(
                        if idx == 0 {
                            LabelStyle::Primary
                        } else {
                            LabelStyle::Secondary
                        },
                        file_map[&pos.file],
                        pos.start..pos.end,
                    );
                    labels.push(l.with_message(msg));
                } else {
                    notes.push(msg.clone());
                }
            }

            let writer = StandardStream::stderr(if opts.no_color {
                ColorChoice::Never
            } else {
                ColorChoice::Always
            });
            term::emit(
                &mut writer.lock(),
                &config,
                &files,
                &Diagnostic::error()
                    .with_message(format!("{}", err.kind))
                    .with_labels(labels)
                    .with_notes(notes),
            )
            .unwrap();
            std::process::exit(1)
        }
    }
}
