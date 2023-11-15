use argh::FromArgs;
use std::{path::PathBuf, str::FromStr};

#[derive(Debug, Default, Clone, Copy)]
/// Solver to use in the pass
pub enum Solver {
    #[default]
    CVC5,
    Z3,
    Boolector,
    Bitwuzla,
}

impl FromStr for Solver {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "z3" => Ok(Solver::Z3),
            "cvc5" => Ok(Solver::CVC5),
            "boolector" => Ok(Solver::Boolector),
            "bitwuzla" => Ok(Solver::Bitwuzla),
            _ => {
                Err(format!("unknown solver: {s}. Known solvers are: z3, cvc5, boolector, bitwuzla"))
            }
        }
    }
}

#[derive(Debug, Default)]
pub enum Backend {
    #[default]
    Verilog,
    Calyx,
}

impl FromStr for Backend {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "calyx" => Ok(Backend::Calyx),
            "verilog" => Ok(Backend::Verilog),
            _ => Err(format!(
                "unknown backend: {s}. Known backends are: calyx, verilog"
            )),
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

    // `gen` options
    /// the output directory to store files generated from external tools
    #[argh(option, long = "out-dir")]
    pub out_dir: Option<PathBuf>,

    // Backend options
    /// backend to use (default: verilog): calyx, verilog
    #[argh(option, long = "backend", default = "Backend::Verilog")]
    pub backend: Backend,
    /// disable generation of slow FSMs in the backend
    #[argh(switch, long = "disable-slow-fsms")]
    pub disable_slow_fsms: bool,
    /// preserves original port names during compilation.
    #[argh(switch, long = "preserve-names")]
    pub preserve_names: bool,

    // Solver specific configuration
    /// solver to use (default: cvc5): cvc5, z3
    #[argh(option, long = "solver", default = "Solver::Z3")]
    pub solver: Solver,
    /// solve assertions separately rather than all at once
    #[argh(switch, long = "discharge-separate")]
    pub discharge_separate: bool,
    /// dump interactions with the solver in the given file
    #[argh(option, long = "dump-solver-log")]
    pub solver_replay_file: Option<String>,
    /// use bitvector encoding for proofs
    #[argh(option, long = "solver-bv")]
    pub solver_bv: Option<u8>,
}
