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
            _ => Err(format!(
                "unknown solver: {s}. Known solvers are: z3, cvc5, boolector, bitwuzla"
            )),
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

    /// print out the IR after every pass
    #[argh(switch, long = "dump-all")]
    pub dump_all: bool,

    /// print out assignments that falsify the constraints
    #[argh(switch, long = "show-models")]
    pub show_models: bool,

    /// path to search for imports
    #[argh(
        option,
        long = "library",
        short = 'l',
        // default = "vec![\"./\".into()]"
        // This is currently not supported by argh 0.1.12, but it has been fixed in the master branch...
    )]
    pub library: Vec<PathBuf>,

    /// only check the program without compilation.
    #[argh(switch, short = 'c', long = "check")]
    pub check: bool,

    /// output the interface.json for the input program
    #[argh(switch, long = "dump-interface")]
    pub dump_interface: bool,

    /// set log level
    #[argh(option, long = "log", default = "log::LevelFilter::Warn")]
    pub log_level: log::LevelFilter,

    /// skip the discharge pass (unsafe)
    #[argh(switch, long = "unsafe-skip-discharge")]
    pub unsafe_skip_discharge: bool,

    // `gen` options
    /// the output directory to store files generated from external tools
    #[argh(option, long = "out-dir")]
    pub out_dir: Option<PathBuf>,

    /// provided bindings (gen config and parameter bindings)
    #[argh(option, long = "bindings")]
    pub bindings: Option<PathBuf>,

    // Backend options
    /// backend to use (default: verilog): calyx, verilog
    #[argh(option, long = "backend", default = "Backend::Verilog")]
    pub backend: Backend,

    /// disable generation of counter-based FSMs in the backend.
    /// The default (non-counter) FSM is represented by a single bit Shift Register counting through the number of states.
    /// However, for components with a large number of states or a large II, it may be more efficient to use a counter-based FSM,
    /// where one counter loops every II states, at which point it increments the state counter.
    #[argh(switch, long = "no-counter-fsms")]
    pub no_counter_fsms: bool,

    /// do not preserve original port names during compilation.
    #[argh(switch, long = "no-preserve-names")]
    pub no_preserve_names: bool,

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
