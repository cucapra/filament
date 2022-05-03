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

    let namespace = frontend::FilamentParser::parse_file(&opts.input)?;
    interval_checking::check(namespace)?;
    Ok(())
}
