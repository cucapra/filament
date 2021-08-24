use filament::{interval_checking, cmdline, errors, frontend};

fn main() -> errors::FilamentResult<()> {
    let opts: cmdline::Opts = argh::from_env();
    let namespace = frontend::FilamentParser::parse_file(&opts.input)?;
    interval_checking::check(namespace)?;
    Ok(())
}
