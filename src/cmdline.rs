use argh::FromArgs;
use std::path::PathBuf;

#[derive(FromArgs, Debug)]
/// The Filament pipeline verifier
pub struct Opts {
    /// path to the primitives library
    #[argh(positional)]
    pub input: PathBuf,

    /// enable IR-based compilation
    #[argh(switch, long = "ir")]
    pub ir: bool,

    /// print out the IR
    #[argh(switch, long = "show-ir")]
    pub show_ir: bool,

    /// print out assignments that falsify the constraints
    #[argh(switch, long = "show-models")]
    pub show_models: bool,

    /// path to search for imports
    #[argh(option, long = "library", short = 'l', default = "\".\".into()")]
    pub library: PathBuf,

    /// only check the program without compilation.
    #[argh(switch, short = 'c', long = "check")]
    pub check: bool,

    /// output the interface.json for the input program
    #[argh(switch, long = "dump-interface")]
    pub dump_interface: bool,

    /// set log level
    #[argh(option, long = "log", default = "log::LevelFilter::Warn")]
    pub log_level: log::LevelFilter,

    /// set toplevel
    #[argh(option, long = "toplevel", default = "\"main\".into()")]
    pub toplevel: String,
}
