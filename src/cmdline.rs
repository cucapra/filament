use argh::FromArgs;
use std::path::PathBuf;

#[derive(FromArgs, Debug)]
/// The Filament pipeline verifier
pub struct Opts {
    /// path to the primitives library
    #[argh(positional)]
    pub input: PathBuf,

    /// print out assignments that falisfy the constraints
    #[argh(switch, long = "show-models")]
    pub show_models: bool,

    /// location of the calyx primitives folder
    #[argh(option, long = "calyx-primitives", default = "\"../calyx\".into()")]
    pub calyx_primitives: PathBuf,

    /// path to search for imports
    #[argh(option, long = "library", short = 'l', default = "\".\".into()")]
    pub library: PathBuf,

    /// only check the program without compilation.
    #[argh(switch, short = 'c', long = "check")]
    pub check: bool,

    /// output the interface.json for the input program
    #[argh(switch, long = "dump-interface")]
    pub dump_interface: bool,
}
