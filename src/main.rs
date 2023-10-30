use calyx_backend::Backend;
use calyx_opt::pass_manager::PassManager;
use fil_ir as ir;
use filament::ir_passes::BuildDomination;
use filament::{cmdline, ir_passes as ip, resolver::Resolver};
use filament::{log_pass, log_time, pass_pipeline};

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
    let mut ir = log_pass! { opts; ir::transform(ns)?, "astconv" };
    pass_pipeline! {opts, ir;
        ip::BuildDomination,
        ip::TypeCheck,
        ip::IntervalCheck,
        ip::PhantomCheck,
        ip::Assume
    }
    if !opts.unsafe_skip_discharge {
        pass_pipeline! {opts, ir; ip::Discharge }
    }
    pass_pipeline! { opts, ir;
        BuildDomination
    };
    ir = log_pass! { opts; ip::Monomorphize::transform(&ir), "monomorphize"};
    pass_pipeline! { opts, ir;
        ip::Simplify,
        ip::AssignCheck,
        ip::BundleElim,
        ip::AssignCheck
    }

    // Return early if we're asked to dump the interface
    if opts.dump_interface {
        ip::DumpInterface::print(&ir);
        return Ok(());
    }

    // Return if we are only checking
    if opts.check {
        return Ok(());
    }
    let calyx = log_time!(
        ip::Compile::compile(ir, opts.disable_slow_fsms, opts.preserve_names),
        "compile"
    );
    match opts.backend {
        cmdline::Backend::Verilog => {
            gen_verilog(calyx).unwrap();
        }
        cmdline::Backend::Calyx => {
            let out = &mut std::io::stdout();
            calyx_ir::Printer::write_context(&calyx, false, out).unwrap();
        }
    }
    Ok(())
}

fn gen_verilog(mut ctx: calyx_ir::Context) -> Result<(), calyx_utils::Error> {
    let pm = PassManager::default_passes()?;
    let backend_conf = calyx_ir::BackendConf {
        synthesis_mode: false,
        enable_verification: false,
        flat_assign: true,
    };
    ctx.bc = backend_conf;
    pm.execute_plan(
        &mut ctx,
        &["all".to_string()],
        &["canonicalize".to_string()],
        false,
    )?;
    let backend = calyx_backend::VerilogBackend;
    backend.run(ctx, calyx_utils::OutputFile::Stdout)
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
