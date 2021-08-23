use filament::{checking::check, cmdline::Opts, errors::FilamentResult, frontend::FilamentParser};

fn main() -> FilamentResult<()> {
    let opts: Opts = argh::from_env();
    let namespace = FilamentParser::parse_file(&opts.input)?;
    check(namespace)?;
    Ok(())
}
