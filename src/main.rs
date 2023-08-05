use filament::{
    binding, cmdline, ir, ir_passes as ip, log_time, pass_pipeline, passes,
    resolver::Resolver, visitor::Checker,
};
use std::time::Instant;

// Prints out the interface for main component in the input program.
fn run(opts: &cmdline::Opts) -> Result<(), u64> {
    // enable tracing
    env_logger::Builder::from_default_env()
        .format_timestamp(None)
        .format_module_path(false)
        .format_target(false)
        .filter_level(opts.log_level)
        .target(env_logger::Target::Stderr)
        .init();

    let ns = match Resolver::from(opts).parse_namespace() {
        Ok(mut ns) => {
            ns.toplevel = opts.toplevel.clone();
            ns
        }
        Err(e) => {
            eprintln!("Error: {e:?}");
            return Err(1);
        }
    };
    log::debug!("{ns}");

    // Construct a binding
    let bind = binding::ProgBinding::try_from(&ns)?;

    // Bind check
    let t = Instant::now();
    passes::BindCheck::check(opts, &ns, &bind)?;
    log::info!("Parameteric Bind check: {}ms", t.elapsed().as_millis());
    drop(bind);

    let mut ir = ir::transform(ns);
    pass_pipeline! {opts, ir;
        ip::BuildDomination,
        ip::TypeCheck,
        ip::IntervalCheck,
        ip::Assume,
        ip::HoistFacts,
        ip::Simplify,
        ip::Discharge
    }
    // Return early if we're asked to dump the interface
    if opts.check {
        return Ok(());
    }
    // TODO(rachit): Once `BundleElim` implements `Visitor`, we can collapse this into
    // one call to `pass_pipeline!`.
    ir = log_time!(ip::Monomorphize::transform(&ir), "monomophization");
    pass_pipeline! {opts, ir; ip::AssignCheck }
    log_time!(ip::BundleElim::do_pass(&mut ir), "bundle-elim");
    pass_pipeline! {opts, ir; ip::AssignCheck }
    if opts.dump_interface {
        ip::DumpInterface::print(&ir);
    }
    log_time!(ip::Compile::compile(ir), "compile");
    Ok(())
}

fn main() {
    let opts: cmdline::Opts = argh::from_env();
    match run(&opts) {
        Ok(_) => (),
        Err(err) => {
            eprintln!("Compilation failed with {err} errors.");
            if !opts.show_models {
                eprintln!("Run with --show-models to generate assignments for failing constraints.");
            }
            std::process::exit(1)
        }
    }
}
