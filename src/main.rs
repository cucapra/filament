use filament::{cmdline, ir, ir_passes as ip, resolver::Resolver};
use filament::{log_time, pass_pipeline};

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

    // Transform AST to IR
    let mut ir = ir::transform(ns)?;
    if opts.dump_after.contains(&"ast-conv".to_string()) {
        ir::Printer::context(&ir, &mut std::io::stdout()).unwrap()
    }

    pass_pipeline! {opts, ir;
        ip::BuildDomination,
        ip::TypeCheck,
        ip::IntervalCheck,
        ip::PhantomCheck,
        ip::Assume,
        ip::HoistFacts,
        ip::Simplify,
        ip::Discharge
    }
    // TODO(rachit): Once `BundleElim` implements `Visitor`, we can collapse this into
    // one call to `pass_pipeline!`.
    ir = log_time!(ip::Monomorphize::transform(&ir), "monomorphize");
    if opts.dump_after.contains(&"monomorphize".to_string()) {
        ir::Printer::context(&ir, &mut std::io::stdout()).unwrap()
    }
    pass_pipeline! {opts, ir; ip::AssignCheck, ip::BundleElim, ip::AssignCheck }
    // Return early if we're asked to dump the interface
    if opts.dump_interface {
        ip::DumpInterface::print(&ir);
        return Ok(());
    }

    // Return if we are only checking
    if opts.check {
        return Ok(());
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
