use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle},
    files::SimpleFiles,
    term::{self, termcolor::ColorChoice, termcolor::StandardStream},
};
use filament::{
    backend, cmdline,
    errors::{self, FilamentResult, WithPos},
    event_checker, frontend, interval_checking, lower,
    visitor::Transform,
};

// Prints out the interface for main component in the input program.
fn dump_interface(
    ns: event_checker::ast::Namespace,
) -> errors::FilamentResult<()> {
    let comp = ns
        .components
        .iter()
        .find(|comp| comp.sig.name == "main")
        .ok_or_else(|| {
            errors::Error::malformed("Program does not define a main component")
        })?;

    // For an interface port like this:
    //      @interface[G, G+5] go_G
    // Generate the JSON information:
    // {
    //   "name": "go_G",
    //   "event": "G",
    //   "delay": 5
    // }
    let interfaces = comp
        .sig
        .interface_signals
        .iter()
        .map(|id| {
            id.delay()
                .concrete()
                .map(|delay| format!( "{{\"name\": \"{}\", \"event\": \"{}\", \"delay\": {delay} }}", id.name, id.event))
                .ok_or_else(|| {
                    errors::Error::malformed(
                        "Interface signal has no concrete delay",
                    )
                })
        })
        .collect::<FilamentResult<Vec<_>>>()?.join(",\n");

    // For each input and output that looks like:
    // @[G+n, G+m] left: 32
    // Generate the JSON information:
    // {
    //   "event": "G",
    //   "name": "left",
    //   "start": n,
    //   "end": m
    // },
    let pd_to_info = |pd: &event_checker::ast::PortDef| {
        pd.liveness
            .within
            .as_offset()
            .map(|(event, st, end)| {
                format!(
                    "{{ \"event\": \"{event}\", \"name\": \"{name}\", \"start\": {st}, \"end\": {end} }}",
                    name = pd.name,
                )
            })
            .ok_or_else(|| {
                errors::Error::malformed(
                    "Delay cannot be converted into concrete start and end",
                )
                .add_note(
                    format!("Delay {} is dynamic", pd.liveness),
                    pd.liveness.copy_span(),
                )
            })
    };
    let inputs = comp
        .sig
        .inputs
        .iter()
        .map(pd_to_info)
        .collect::<FilamentResult<Vec<_>>>()?
        .join(",\n");
    let outputs = comp
        .sig
        .outputs
        .iter()
        .map(pd_to_info)
        .collect::<FilamentResult<Vec<_>>>()?
        .join(",\n");

    // Look ma, a JSON serializer!
    println!(
        "{{\n\"interfaces\": [\n{interfaces}\n],\n\"inputs\": [\n{inputs}\n],\n\"outputs\": [\n{outputs}\n]\n}}",
    );

    Ok(())
}

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
    let ns = event_checker::check_and_transform(ns)?;
    log::info!("Event check:\n{ns}");
    interval_checking::check(&ns)?;

    // Dump the interface
    if opts.dump_interface {
        return dump_interface(ns);
    }

    // Lowering pipeline
    if !opts.check {
        let ns = lower::CompileInvokes::transform(ns)?;
        log::info!("Lowering:\n{ns}");
        // interval_checking::check(&ns)?;
        backend::compile(ns, opts)?;
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
