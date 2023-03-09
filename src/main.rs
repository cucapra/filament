use filament::{
    backend, binding, cmdline, passes,
    resolver::Resolver,
    visitor::{Checker, Transform},
};
use std::time::Instant;

// Prints out the interface for main component in the input program.
fn run(opts: &cmdline::Opts) -> Result<(), u64> {
    // enable tracing
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        // .filter_level(opts.log_level)
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

    // Construct a binding
    let bind = binding::ProgBinding::try_from(&ns)?;

    // Bind check
    let t = Instant::now();
    passes::BindCheck::check(&opts, &ns, &bind)?;
    log::info!("Parameteric Bind check: {}ms", t.elapsed().as_millis());

    // Interval checking
    let t = Instant::now();
    passes::IntervalCheck::check(&opts, &ns, &bind)?;
    log::info!("Interval check: {}ms", t.elapsed().as_millis());

    // User-level @phantom ports
    let t = Instant::now();
    passes::PhantomCheck::check(&opts, &ns, &bind)?;
    log::info!("Phantom check: {}ms", t.elapsed().as_millis());

    // Monomorphize the program.
    let t = Instant::now();
    let ns = passes::Monomorphize::transform(ns);
    log::info!("Monomorphize: {}ms", t.elapsed().as_millis());
    log::trace!("{ns}");

    // Rebuild the binding
    let bind = binding::ProgBinding::try_from(&ns)?;

    // Monomorphic Bind check
    let t = Instant::now();
    passes::BindCheck::check(&opts, &ns, &bind)?;
    log::info!("Monomorphoic Bind check: {}ms", t.elapsed().as_millis());

    // Monomorphic Interval checking
    let t = Instant::now();
    passes::IntervalCheck::check(&opts, &ns, &bind)?;
    log::info!("Monomorphoic Interval check: {}ms", t.elapsed().as_millis());

    // Max state calculation
    let states = passes::MaxStates::check(&opts, &ns, &bind)?;

    if opts.dump_interface {
        passes::DumpInterface::transform_unwrap(ns, states.max_states);
        return Ok(());
    }

    // Return early if we're asked to dump the interface
    if opts.check {
        return Ok(());
    }
    // Lowering
    let t = Instant::now();
    let Some(ns) =
        passes::Lower::transform_unwrap(ns, states.max_states) else {
            return Err(1);
        };
    log::info!("Lowering: {}ms", t.elapsed().as_millis());
    log::trace!("{ns}");

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
            eprintln!("Compilation failed with {err} errors.");
            std::process::exit(1)
        }
    }
}
