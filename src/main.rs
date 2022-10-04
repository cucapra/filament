use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Instant,
};

use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle},
    files::SimpleFiles,
    term::{self, termcolor::ColorChoice, termcolor::StandardStream},
};
use filament::{
    backend, bind_check, cmdline, dump_interface,
    errors::{self, FilamentResult},
    event_checker, frontend, interval_checking, lower, phantom_check,
    visitor::Transform,
};

/// Parse a completely resovled namespace from an input file. Recursively parses all imported files and resolves
/// the paths for externs.
fn parse_namespace(
    opts: &cmdline::Opts,
) -> FilamentResult<frontend::ast::Namespace> {
    let mut ns = frontend::FilamentParser::parse_file(&opts.input)?;

    // Files that have already been imported
    let mut already_imported = HashSet::new();

    // Resolve import either using opts.library or relative the parent directory of the input file.
    let resolve_import =
        |imp: &String, dir: &Path| -> FilamentResult<PathBuf> {
            // Resolve against the library path
            let mut lib_base = opts.library.clone();
            lib_base.push(imp);
            if lib_base.exists() {
                return Ok(lib_base);
            }
            // Attempt to resove against base
            let mut cur_base = dir.to_path_buf();
            cur_base.push(imp);
            if cur_base.exists() {
                Ok(cur_base)
            } else {
                Err(errors::Error::misc(format!(
                "Could not resolve import path: {}. Neither {} nor {} exist.",
                imp,
                lib_base.display(),
                cur_base.display()
            )))
            }
        };

    // Get absolute path for a relative path `ext_path` wrt to the parent of the file `file_path`.
    let absolute = |ext_path: String, base: &Path| -> String {
        let ext: PathBuf = ext_path.into();
        if ext.is_relative() {
            let mut base = base.to_path_buf();
            base.push(ext);
            base
        } else {
            ext
        }
        .to_string_lossy()
        .to_string()
    };

    // Get the parent of a given file
    let parent = |p: &Path| -> PathBuf {
        let mut p = p.to_path_buf();
        p.pop();
        p
    };

    // Extern are resolved to thier absolute path relative to the input file.
    let base = parent(&opts.input);
    let mut imports: Vec<PathBuf> = ns
        .imports
        .drain(..)
        .filter_map(|s| {
            let f = resolve_import(&s, &base);
            if let Ok(file) = f {
                if !already_imported.contains(&file) {
                    already_imported.insert(file.clone());
                    Some(Ok(file))
                } else {
                    None
                }
            } else {
                Some(f)
            }
        })
        .collect::<FilamentResult<_>>()?;

    ns.externs = ns
        .externs
        .drain(..)
        .map(|(p, imps)| (absolute(p, &base), imps))
        .collect();

    while let Some(path) = imports.pop() {
        let mut imp = frontend::FilamentParser::parse_file(&path)?;
        let base = parent(&path);
        imp.components.append(&mut ns.components);
        ns.components = imp.components;
        ns.externs.extend(
            imp.externs
                .into_iter()
                .map(|(p, imps)| (absolute(p, &base), imps)),
        );
        let base = parent(&path);
        imports.extend(
            imp.imports
                .into_iter()
                .filter_map(|s| {
                    let f = resolve_import(&s, &base);
                    if let Ok(file) = f {
                        if !already_imported.contains(&file) {
                            already_imported.insert(file.clone());
                            Some(Ok(file))
                        } else {
                            None
                        }
                    } else {
                        Some(f)
                    }
                })
                .collect::<FilamentResult<Vec<_>>>()?,
        );
    }
    // eprintln!("Imported: {:#?}", already_imported);
    Ok(ns)
}

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

    let ns = parse_namespace(opts)?;

    // Run the compilation pipeline
    let t = Instant::now();
    let ns = event_checker::check_and_transform(ns)?;
    log::info!("Event check: {}ms", t.elapsed().as_millis());
    log::trace!("{ns}");

    // Bind check
    let t = Instant::now();
    let ns = bind_check::check(ns)?;
    log::info!("Bind check: {}ms", t.elapsed().as_millis());

    // Interval checking
    let t = Instant::now();
    let mut ns = interval_checking::check(ns)?;
    log::info!("Interval check: {}ms", t.elapsed().as_millis());

    // User-level @phantom ports
    ns = phantom_check::PhantomCheck::transform(ns)?;
    log::info!("Phantom check: {}ms", t.elapsed().as_millis());

    if opts.dump_interface {
        // Lowering
        let t = Instant::now();
        ns = lower::CompileInvokes::<false>::transform(ns)?;
        log::info!("Lowering: {}ms", t.elapsed().as_millis());
        log::info!("{ns}");
        dump_interface::DumpInterface::transform(ns)?;
    } else if !opts.check {
        // Lowering
        let t = Instant::now();
        ns = lower::CompileInvokes::<true>::transform(ns)?;
        log::info!("Lowering: {}ms", t.elapsed().as_millis());
        log::info!("{ns}");

        // Compilation
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
