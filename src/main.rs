use filament::{cmdline, errors, frontend, interval_checking};

fn main() -> errors::FilamentResult<()> {
    let opts: cmdline::Opts = argh::from_env();
    let namespace = frontend::FilamentParser::parse_file(&opts.input)?;
    interval_checking::check(namespace)?;
    Ok(())
}
