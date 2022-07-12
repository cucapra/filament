use std::path::{Path, PathBuf};

use filament::{
    backend, cmdline, errors, event_checker, frontend, interval_checking,
    lower, visitor::Transform, interface_infer,
};

fn main() -> errors::FilamentResult<()> {
    let opts: cmdline::Opts = argh::from_env();

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
    let ns = interface_infer::InterfaceInfer::transform(ns)?;
    log::info!("Interface Infer:\n{ns}");
    interval_checking::check(&ns)?;

    // Lowering pipeline
    if !opts.check {
        let ns = lower::CompileInvokes::transform(ns)?;
        log::info!("Lowering:\n{ns}");
        interval_checking::check(&ns)?;
        backend::compile(ns, &opts)?;
    }
    Ok(())
}
