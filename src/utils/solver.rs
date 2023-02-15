use crate::core::{self, Time, TimeSub};
use crate::diagnostics::{Diagnostics, InfoIdx};
use crate::errors::{Error, FilamentResult};
use crate::utils::GPosIdx;
use itertools::Itertools;
use rsmt2::{SmtConf, Solver};

#[derive(Clone)]
pub struct ShareConstraint {
    /// Delay bounded by the share constraint
    /// XXX: We store the event binding so that we can get the position of the binding.
    event_bind: core::EventBind,
    /// The events used to compute the minimum of start times
    starts: Vec<Time>,
    /// The (event, delay) to compute the max of start times
    ends: Vec<(Time, TimeSub)>,
    /// Additional error information
    notes: Vec<InfoIdx>,
}

impl From<core::EventBind> for ShareConstraint {
    fn from(bind: core::EventBind) -> Self {
        Self {
            starts: vec![],
            ends: vec![],
            event_bind: bind,
            notes: vec![],
        }
    }
}

impl ShareConstraint {
    pub fn add_note(&mut self, info: InfoIdx) {
        self.notes.push(info);
    }

    pub fn add_bind_info(
        &mut self,
        start: Time,
        end: (Time, TimeSub),
        pos: GPosIdx,
        diag: &mut Diagnostics,
    ) {
        self.add_note(diag.add_info(
            format!("Invocation active during [{start}, {}+{})", end.0, end.1),
            pos,
        ));
        self.starts.push(start);
        self.ends.push(end);
    }
}
impl std::fmt::Display for ShareConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let min = self.starts.iter().map(|t| t.to_string()).join(", ");
        let max = self
            .ends
            .iter()
            .map(|(t, d)| format!("{} + {}", t, d))
            .join(", ");
        let delay = &self.event_bind.delay;
        write!(f, "{delay} >= max({max}) - min({min})")
    }
}
impl From<ShareConstraint> for SExp {
    fn from(sh: ShareConstraint) -> Self {
        let min = sh
            .starts
            .into_iter()
            .map(SExp::from)
            .reduce(|a, b| SExp(format!("(min {} {})", a, b)))
            .unwrap();
        let max = sh
            .ends
            .into_iter()
            .map(|(t, d)| {
                SExp(format!("(+ {} {})", SExp::from(t), SExp::from(d)))
            })
            .reduce(|a, b| SExp(format!("(max {} {})", a, b)))
            .unwrap();

        SExp(format!(
            "(>= {} (- {max} {min}))",
            SExp::from(sh.event_bind.delay)
        ))
    }
}

impl From<ShareConstraint> for Error {
    fn from(sh: ShareConstraint) -> Self {
        let msg = format!("Cannot prove: {}", sh);
        let mut err = Error::malformed(msg);
        err.notes = sh.notes;
        err
    }
}

/// A string that semantically represents an S-expression
#[derive(Clone)]
pub struct SExp(pub String);
impl std::fmt::Display for SExp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for SExp {
    fn from(n: u64) -> Self {
        SExp(n.to_string())
    }
}

fn define_prelude<P>(solver: &mut Solver<P>) -> FilamentResult<()> {
    solver.define_fun(
        "max",
        &[("x", "Int"), ("y", "Int")],
        "Int",
        "(ite (< x y) y x)",
    )?;
    solver.define_fun(
        "min",
        &[("x", "Int"), ("y", "Int")],
        "Int",
        "(ite (> x y) y x)",
    )?;
    solver.define_fun(
        "abs",
        &[("x", "Int")],
        "Int",
        "(ite (< x 0) (- x) x)",
    )?;
    Ok(())
}

pub struct FilSolver {
    s: Solver<()>,
}

impl FilSolver {
    pub fn new() -> FilamentResult<Self> {
        let conf = SmtConf::default_z3();
        // Disable this because it doesn't seem to work with activation literals
        // conf.check_success();

        let mut solver = conf.spawn(())?;
        solver.produce_models()?;
        solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

        define_prelude(&mut solver)?;
        Ok(Self { s: solver })
    }

    pub fn prove(
        &mut self,
        vars: impl Iterator<Item = core::Id>,
        assumes: Vec<core::Constraint>,
        asserts: Vec<core::Constraint>,
        sharing: Vec<ShareConstraint>,
        diag: &mut Diagnostics,
    ) {
        if asserts.is_empty() {
            return;
        }

        eprintln!("Asserts: {}", asserts.len());
        let asserts = asserts.into_iter().unique().collect_vec();
        eprintln!("Unique asserts: {}", asserts.len());
        self.s.push(1).unwrap();
        // Define all the constants
        for var in vars {
            log::trace!("Declaring constant {}", var);
            self.s.declare_const(var.to_string(), "Int").unwrap();
        }

        // Define assumptions on constraints
        for assume in assumes {
            log::trace!("Assuming {}", assume);
            let sexp: SExp = assume.into();
            self.s.assert(format!("{}", sexp)).unwrap();
        }

        for fact in asserts {
            if !self.check_fact(fact.clone()) {
                diag.add_error(fact.into());
            }
        }
        for share in sharing {
            if !self.check_fact(share.clone()) {
                diag.add_error(share.into());
            }
        }

        self.s.pop(1).unwrap();
    }

    fn check_fact(&mut self, fact: impl Into<SExp>) -> bool {
        let sexp = fact.into();
        self.s.push(1).unwrap();
        let formula = format!("(not {})", sexp);
        log::trace!("Assert {}", formula);
        self.s.assert(formula).unwrap();
        // Check that the assertion was unsatisfiable
        let unsat = !self.s.check_sat().unwrap();
        if !unsat {
            log::trace!("MODEL: {:?}", self.s.get_model().unwrap());
        }
        self.s.pop(1).unwrap();
        unsat
    }
}
