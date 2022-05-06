use filament::{cmdline, errors, frontend, interval_checking};

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
    while let Some(file) = imports.pop() {
        let path: std::path::PathBuf = file.into();
        let imp = frontend::FilamentParser::parse_file(&path)?;
        ns.components.extend(imp.components.into_iter());
        ns.signatures.extend(imp.signatures.into_iter());
        imports.extend(imp.imports.into_iter());
    }
    interval_checking::check(&ns)?;
    Ok(())
}
