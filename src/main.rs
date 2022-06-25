use std::path::{Path, PathBuf};

use filament::{
    backend, cmdline, errors, event_checker, frontend, interval_checking, lower,
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
    let mut imports: Vec<String> = ns.imports.drain(..).collect();
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
    while let Some(file) = imports.pop() {
        let path: PathBuf = file.into();
        let imp = frontend::FilamentParser::parse_file(&path)?;
        ns.components.extend(imp.components.into_iter());
        ns.externs.extend(
            imp.externs
                .into_iter()
                .map(|(p, imps)| (absolute(p, &path), imps)),
        );
        imports.extend(imp.imports.into_iter());
    }

    // Run the compilation pipeline
    let ns = event_checker::check_and_transform(ns)?;
    interval_checking::check(&ns)?;
    let ns = lower::lower_invokes(ns)?;
    interval_checking::check(&ns)?;
    if !opts.check {
        backend::compile(ns, &opts)?;
    }
    Ok(())
}
