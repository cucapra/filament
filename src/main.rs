use std::{collections::HashMap, time::Instant};

use codespan_reporting::{
    diagnostic::{Diagnostic, Label, LabelStyle},
    files::SimpleFiles,
    term::{self, termcolor::ColorChoice, termcolor::StandardStream},
};
use filament::{
    backend, bind_check, cmdline, dump_interface, errors, interval_checking,
    lower, max_states, phantom_check, resolver::Resolver,
    utils::GlobalPositionTable, visitor::Transform,
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

    let ns = Resolver::from(opts).parse_namespace()?;

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
    (ns, _) = phantom_check::PhantomCheck::transform(ns, ())?;
    log::info!("Phantom check: {}ms", t.elapsed().as_millis());

    let (mut ns, states) = max_states::MaxStates::transform(ns, ())?;
    log::info!("Max states: {:?}", states.max_states);

    // Return early if we're asked to dump the interface
    if opts.dump_interface {
        dump_interface::DumpInterface::transform(ns, states.max_states)?;
        return Ok(());
    } else if opts.check {
        return Ok(());
    }

    // Lowering
    let t = Instant::now();
    (ns, _) = lower::CompileInvokes::transform(ns, states.max_states)?;
    log::info!("Lowering: {}ms", t.elapsed().as_millis());
    log::info!("{ns}");

    // Compilation
    let t = Instant::now();
    backend::compile(ns, opts)?;
    log::info!("Compilation: {}ms", t.elapsed().as_millis());

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
                let idx = files.add(name.to_string().clone(), file);
                file_map.insert(name, idx);
            }

            // Construct mapping from files to indices
            let table = GlobalPositionTable::as_ref();
            let mut labels = vec![];
            let mut notes = vec![];
            for (idx, (msg, pos)) in err.notes.iter().enumerate() {
                if let Some(p) = pos.into_option() {
                    let pos = table.get_pos(p.0);
                    let (file, _) = table.get_file_info(p.0);
                    let l = Label::new(
                        if idx == 0 {
                            LabelStyle::Primary
                        } else {
                            LabelStyle::Secondary
                        },
                        file_map[file],
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
