use filament::{
    backend, binding, cmdline, ir,
    ir_passes::{self},
    ir_visitor::Visitor,
    passes::{self, Pass},
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

    if opts.ir {
        let mut ir = ir::transform(ns);
        ir_passes::TypeCheck::do_pass(opts, &mut ir)?;
        ir_passes::IntervalCheck::do_pass(opts, &mut ir)?;
        ir_passes::Assume::do_pass(opts, &mut ir)?;
        ir_passes::HoistFacts::do_pass(opts, &mut ir)?;
        ir_passes::Simplify::do_pass(opts, &mut ir)?;
        if opts.show_ir {
            println!("before mono");
            println!("====================================================");
            ir::Printer::context(&ir, &mut std::io::stdout()).unwrap();
            println!("====================================================");
        }
        if !opts.check {
            ir = ir_passes::Monomorphize::transform(&ir);
        }
        if opts.show_ir {
            println!("after mono");
            println!("====================================================");
            ir::Printer::context(&ir, &mut std::io::stdout()).unwrap();
            println!("====================================================");
        }
        ir_passes::Discharge::do_pass(opts, &mut ir)?;
        ir_passes::BundleElim::do_pass(&mut ir);
        if opts.show_ir {
            println!("after bundle-elim");
            println!("====================================================");
            ir::Printer::context(&ir, &mut std::io::stdout()).unwrap();
            println!("====================================================");
        }
        ir_passes::Compile::compile(ir);
        return Ok(());
    }

    // Add default assumption constraints
    let t = Instant::now();
    let ns = passes::Assume::transform(ns);
    log::info!("Assume: {}ms", t.elapsed().as_millis());
    log::debug!("{ns}");

    // Construct a binding
    let bind = binding::ProgBinding::try_from(&ns)?;

    // Interval checking
    let t = Instant::now();
    passes::IntervalCheck::check(opts, &ns, &bind)?;
    log::info!("Interval check: {}ms", t.elapsed().as_millis());

    // User-level @phantom ports
    let t = Instant::now();
    passes::PhantomCheck::check(opts, &ns, &bind)?;
    log::info!("Phantom check: {}ms", t.elapsed().as_millis());

    // Monomorphize the program.
    let t = Instant::now();
    let ns = passes::Monomorphize::transform(ns);
    log::info!("Monomorphize: {}ms", t.elapsed().as_millis());
    log::debug!("{ns}");

    // Bundle elimination
    let t = Instant::now();
    let ns = passes::BundleElim::transform(ns);
    log::info!("Bundle elimination: {}ms", t.elapsed().as_millis());
    log::debug!("{ns}");

    // Rebuild the binding
    let bind = binding::ProgBinding::try_from(&ns)?;

    // Monomorphic Bind check
    let t = Instant::now();
    passes::BindCheck::check(opts, &ns, &bind)?;
    log::info!("Monomorphoic Bind check: {}ms", t.elapsed().as_millis());

    // Monomorphic Interval checking
    let t = Instant::now();
    passes::IntervalCheck::check(opts, &ns, &bind)?;
    log::info!("Monomorphoic Interval check: {}ms", t.elapsed().as_millis());

    // Max state calculation
    let states = passes::MaxStates::check(opts, &ns, &bind)?;

    if opts.dump_interface {
        passes::DumpInterface::print(&ns, &states.max_states);
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
    log::debug!("{ns}");

    // Compilation
    let t = Instant::now();
    backend::compile(ns);
    log::info!("Compilation: {}ms", t.elapsed().as_millis());

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
