use std::path::PathBuf;

/// The filament gen tool
#[derive(argh::FromArgs)]
pub struct Opts {
    /// location of the tool description
    #[argh(option, short = 't')]
    pub tool: String,
    /// location of the manifest file
    #[argh(option, short = 'm')]
    pub manifest: String,
    /// logging level
    #[argh(
        option,
        short = 'l',
        long = "log",
        default = "log::LevelFilter::Warn"
    )]
    pub log_level: log::LevelFilter,
    /// output directory for the generated files
    #[argh(option, short = 'o')]
    pub out_dir: PathBuf,
    /// print out commands to run instead of running them
    #[argh(switch, short = 'n')]
    pub dry_run: bool,
}
