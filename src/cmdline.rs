use argh::FromArgs;
use std::path::PathBuf;

#[derive(FromArgs, Debug)]
/// The Filament pipeline verifier
pub struct Opts {
    /// path to the primitives library
    #[argh(positional)]
    pub input: PathBuf,

    /// enable verbose printing
    #[argh(option, long = "log", default = "log::LevelFilter::Warn")]
    pub log_level: log::LevelFilter,

    /// only check the program without compilation
    #[argh(switch, short = 'c', long = "check")]
    pub check: bool,
}
