use filament::{
    backend, cmdline, passes,
    resolver::Resolver,
    visitor::{Checker, Transform},
};
use std::time::Instant;

// Prints out the interface for main component in the input program.
fn run(opts: &cmdline::Opts) -> Result<(), u64> {
    // enable tracing
    env_logger::Builder::new()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(opts.log_level)
        .target(env_logger::Target::Stderr)
        .init();

    let ns = match Resolver::from(opts).parse_namespace() {
        Ok(ns) => ns,
        Err(e) => {
            eprintln!("Error: {e:?}");
            return Err(1);
        }
    };
    log::trace!("{ns}");

    // Bind check
    let t = Instant::now();
    passes::BindCheck::check(&ns)?;
    log::info!("Parameteric Bind check: {}ms", t.elapsed().as_millis());

    // Interval checking
    let t = Instant::now();
    passes::IntervalCheck::check(&ns)?;
    log::info!("Interval check: {}ms", t.elapsed().as_millis());

    // User-level @phantom ports
    passes::PhantomCheck::check(&ns)?;
    log::info!("Phantom check: {}ms", t.elapsed().as_millis());

    // Return early if we're asked to dump the interface
    if opts.check {
        return Ok(());
    }

    // Monomorphize the program.
    let t = Instant::now();
    let ns = passes::Monomorphize::transform(ns);
    log::info!("Monomorphize: {}ms", t.elapsed().as_millis());
    log::trace!("{ns}");

    // Monomorphic Bind check
    let t = Instant::now();
    passes::BindCheck::check(&ns)?;
    log::info!("Monomorphoic Bind check: {}ms", t.elapsed().as_millis());

    if opts.dump_interface {
        let states = passes::MaxStates::check(&ns)?;
        passes::DumpInterface::transform_unwrap(ns, states.max_states);
        return Ok(());
    }

    // Monomorphic Interval checking
    let t = Instant::now();
    passes::IntervalCheck::check(&ns)?;
    log::info!("Monomorphoic Interval check: {}ms", t.elapsed().as_millis());

    // Max state calculation
    let states = passes::MaxStates::check(&ns)?;

    // Lowering
    let t = Instant::now();
    let Some(ns) =
        passes::CompileInvokes::transform_unwrap(ns, states.max_states) else {
            return Err(1);
        };
    log::info!("Lowering: {}ms", t.elapsed().as_millis());
    log::info!("{ns}");

    // Compilation
    let t = Instant::now();
    backend::compile(ns, opts);
    log::info!("Compilation: {}ms", t.elapsed().as_millis());

    Ok(())
}

fn main() {
    let opts: cmdline::Opts = argh::from_env();
    match run(&opts) {
        Ok(_) => (),
        Err(err) => {
            eprintln!("Escaped error: {err:?}");
            std::process::exit(1)
        }
    }
}
