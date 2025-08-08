use calyx_backend::Backend;
use calyx_opt::pass_manager::PassManager;
use fil_gen::GenConfig;
use fil_ir as ir;
use filament::ir_passes::BuildDomination;
use filament::{ast_pass_pipeline, ir_pass_pipeline, log_pass, log_time};
use filament::{
    ast_passes as ap, ast_visitor::Visitor as AstVisitor, cmdline,
    ir_passes as ip, ir_visitor::Visitor as IrVisitor, resolver::Resolver,
};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;

#[derive(Deserialize, Default, Debug)]
#[serde(default)]
/// Contains the bindings that are provided by the user.
pub struct ProvidedBindings {
    /// Gen configuration variables
    r#gen: GenConfig,
    /// Parameters to give to the components
    params: HashMap<String, Vec<u64>>,
}

/// Helper function to add IR pass names to the collection
fn add_ir_pass<P: IrVisitor>(pass_names: &mut Vec<String>) {
    pass_names.push(P::name().to_string());
}

/// Helper function to add AST pass names to the collection
fn add_ast_pass<P: AstVisitor>(pass_names: &mut Vec<String>) {
    pass_names.push(P::name().to_string());
}

/// Collects all known pass names by calling name() on each pass type
/// and including hardcoded pass names from log_pass! calls
fn collect_known_pass_names() -> Vec<String> {
    let mut pass_names = vec![
        // Hardcoded pass names from log_pass! calls
        "astconv".to_string(),
        "monomorphize".to_string(),
        // Pass name from log_time! call
        "compile".to_string(),
    ];

    // IR pass names - call name() method on each pass type
    add_ir_pass::<ip::Assumptions>(&mut pass_names);
    add_ir_pass::<ip::BuildDomination>(&mut pass_names);
    add_ir_pass::<ip::TypeCheck>(&mut pass_names);
    add_ir_pass::<ip::IntervalCheck>(&mut pass_names);
    add_ir_pass::<ip::PhantomCheck>(&mut pass_names);
    add_ir_pass::<ip::InferAssumes>(&mut pass_names);
    add_ir_pass::<ip::Discharge>(&mut pass_names);
    add_ir_pass::<ip::FSMAttributes>(&mut pass_names);
    add_ir_pass::<ip::Simplify>(&mut pass_names);
    add_ir_pass::<ip::AssignCheck>(&mut pass_names);
    add_ir_pass::<ip::BundleElim>(&mut pass_names);

    // AST pass names
    add_ast_pass::<ap::TopLevel>(&mut pass_names);

    pass_names.sort();
    pass_names.dedup();
    pass_names
}

/// Validates that all --dump-after pass names are known passes
fn validate_dump_after_passes(opts: &cmdline::Opts) -> Result<(), String> {
    if opts.dump_after.is_empty() {
        return Ok(());
    }

    let known_passes = collect_known_pass_names();
    let mut unknown_passes = Vec::new();

    for pass_name in &opts.dump_after {
        if !known_passes.contains(pass_name) {
            unknown_passes.push(pass_name.clone());
        }
    }

    if !unknown_passes.is_empty() {
        let error_msg = format!(
            "Unknown pass name(s): {}\nKnown passes are: {}",
            unknown_passes.join(", "),
            known_passes.join(", ")
        );
        return Err(error_msg);
    }

    Ok(())
}

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

    // Validate --dump-after pass names
    if let Err(e) = validate_dump_after_passes(opts) {
        eprintln!("Error: {}", e);
        return Err(1);
    }

    // Load the provided bindings
    let provided_bindings: ProvidedBindings = opts
        .bindings
        .as_ref()
        .map(|path| toml::from_str(&fs::read_to_string(path).unwrap()).unwrap())
        .unwrap_or_default();

    let mut ns = match Resolver::from(opts).parse_namespace() {
        Ok(ns) => ns,
        Err(e) => {
            eprintln!("Error: {e:?}");
            return Err(1);
        }
    };

    ast_pass_pipeline! { opts, ns; ap::TopLevel };

    // Set the parameter bindings for the top-level component
    if let Some(main) = ns.toplevel() {
        ns.bindings = provided_bindings
            .params
            .get(main)
            .cloned()
            .unwrap_or_default();
    }

    // Initialize the generator
    let mut gen_exec = if ns.requires_gen() {
        if opts.out_dir.is_none()
            && matches!(opts.backend, cmdline::Backend::Calyx)
        {
            log::warn!(concat!(
                "Generated calyx program will NOT compile because it depends ",
                "on generated files. Please provide an output directory using ",
                "`--out-dir <dir>` to store the generated files."
            ))
        }
        Some(ns.init_gen(opts.out_dir.clone(), provided_bindings.r#gen))
    } else {
        None
    };

    // Transform AST to IR
    let mut ir = log_pass! { opts; ir::transform(ns)?, "astconv" };
    ir_pass_pipeline! {opts, ir;
        ip::Assumptions,
        ip::BuildDomination,
        ip::TypeCheck,
        ip::IntervalCheck,
        ip::PhantomCheck,
        ip::InferAssumes
    }
    if !opts.unsafe_skip_discharge {
        ir_pass_pipeline! {opts, ir; ip::Discharge }
    }
    ir_pass_pipeline! { opts, ir;
        BuildDomination
    };
    ir = log_pass! { opts; ip::Monomorphize::transform(&ir, &mut gen_exec), "monomorphize"};
    ir_pass_pipeline! { opts, ir;
        ip::FSMAttributes,
        ip::Simplify,
        ip::AssignCheck,
        ip::BundleElim,
        ip::AssignCheck
    }
    // type check again before lowering
    ir_pass_pipeline! {opts, ir;
        ip::Assumptions,
        ip::BuildDomination,
        ip::TypeCheck,
        ip::IntervalCheck,
        ip::PhantomCheck,
        ip::InferAssumes
    }
    if !opts.unsafe_skip_discharge {
        ir_pass_pipeline! {opts, ir; ip::Discharge }
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
    let calyx =
        log_time!(ip::Compile::compile(ir, !opts.no_preserve_names), "compile");
    match opts.backend {
        cmdline::Backend::Verilog => {
            gen_verilog(calyx).unwrap();
        }
        cmdline::Backend::Calyx => {
            let out = &mut std::io::stdout();
            calyx_ir::Printer::write_context(&calyx, false, out).unwrap();
        }
    }

    // Drop the generator executor after the execution finishes to ensure that
    // Calyx has access to the generated file.
    drop(gen_exec);
    Ok(())
}

fn gen_verilog(mut ctx: calyx_ir::Context) -> Result<(), calyx_utils::Error> {
    let pm = PassManager::default_passes()?;
    let backend_conf = calyx_ir::BackendConf {
        synthesis_mode: false,
        enable_verification: false,
        flat_assign: true,
        emit_primitive_extmodules: false,
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
                eprintln!(
                    "Run with --show-models to generate assignments for failing constraints."
                );
            }
            std::process::exit(1)
        }
    }
}
