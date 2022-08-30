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

    /// location of the calyx primitives folder
    #[argh(option, long = "calyx-primitives", default = "\"../calyx\".into()")]
    pub calyx_primitives: PathBuf,

    /// path to search for imports
    #[argh(option, long = "library", short = 'l', default = "\".\".into()")]
    pub library: PathBuf,

    /// disable colorized output
    #[argh(switch, long = "no-color")]
    pub no_color: bool,

    /// only check the program without compilation.
    #[argh(switch, short = 'c', long = "check")]
    pub check: bool,

    /// output the interface.json for the input program
    #[argh(switch, long = "dump-interface")]
    pub dump_interface: bool,
}
