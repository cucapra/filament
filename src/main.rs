use filament::{cmdline::Opts, errors::FilamentResult, frontend::FilamentParser};

fn main() -> FilamentResult<()> {
    let opts: Opts = argh::from_env();
    FilamentParser::parse_file(&opts.input)?;

    println!("Hello, world!");

    Ok(())
}
