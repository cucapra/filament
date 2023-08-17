use argh::FromArgs;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Default)]
/// Solver to use in the pass
pub enum Solver {
    #[default]
    CVC5,
    Z3,
}

impl FromStr for Solver {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "z3" => Ok(Solver::Z3),
            "cvc5" => Ok(Solver::CVC5),
            _ => {
                Err(format!("unknown solver: {s}. Known solvers are: z3, cvc5"))
            }
        }
    }
}

#[derive(FromArgs, Debug)]
/// The Filament pipeline verifier
pub struct Opts {
    /// path to the primitives library
    #[argh(positional)]
    pub input: PathBuf,

    /// print out the IR
    #[argh(option, long = "dump-after")]
    pub dump_after: Vec<String>,

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

    /// skip the discharge pass (unsafe)
    #[argh(switch, long = "unsafe-skip-discharge")]
    pub unsafe_skip_discharge: bool,

    // Solver specific configuration
    /// solver to use (default: z3)
    #[argh(option, long = "solver", default = "Solver::CVC5")]
    pub solver: Solver,

    /// dump interactions with the solver in the given file
    #[argh(option, long = "dump-solver-log")]
    pub solver_replay_file: Option<String>,
}
