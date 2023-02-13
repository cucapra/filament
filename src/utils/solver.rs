use crate::core::{self, ConcTime, TimeSub};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use itertools::Itertools;
use rsmt2::{SmtConf, Solver};

#[derive(Clone)]
pub struct ShareConstraints {
    /// Delay bounded by the share constraint
    /// XXX: We store the event binding so that we can get the position of the binding.
    event_bind: core::EventBind,
    /// The events used to compute the minimum of start times
    starts: Vec<ConcTime>,
    /// The (event, delay) to compute the max of start times
    ends: Vec<(ConcTime, TimeSub)>,
    /// Additional error information
    notes: Vec<(String, GPosIdx)>,
}

impl From<core::EventBind> for ShareConstraints {
    fn from(bind: core::EventBind) -> Self {
        Self {
            starts: vec![],
            ends: vec![],
            event_bind: bind,
            notes: vec![],
        }
    }
}

impl ShareConstraints {
    pub fn add_note<S: Into<String>>(&mut self, msg: S, pos: GPosIdx) {
        self.notes.push((msg.into(), pos));
    }

    pub fn notes(self) -> Vec<(String, GPosIdx)> {
        self.notes
    }

    pub fn add_bind_info(
        &mut self,
        start: ConcTime,
        end: (ConcTime, TimeSub),
        pos: GPosIdx,
    ) {
        self.add_note(
            format!("Invocation active during [{start}, {}+{})", end.0, end.1),
            pos,
        );
        self.starts.push(start);
        self.ends.push(end);
    }
}
impl std::fmt::Display for ShareConstraints {
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
impl From<ShareConstraints> for SExp {
    fn from(sh: ShareConstraints) -> Self {
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
        let mut conf = SmtConf::default_z3();
        // Immediately checks if the command to z3 succeeded.
        conf.check_success();

        let mut solver = conf.spawn(())?;
        solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

        define_prelude(&mut solver)?;
        Ok(Self { s: solver })
    }

    pub fn prove(
        &mut self,
        vars: impl Iterator<Item = core::Id>,
        assumes: Vec<core::Constraint>,
        asserts: impl Iterator<Item = core::Constraint>,
        sharing: Vec<ShareConstraints>,
    ) -> FilamentResult<()> {
        // Locally simplify as many asserts as possible
        /* let asserts = asserts
        .into_iter()
        .flat_map(|con| con.simplify())
        .collect::<Vec<_>>(); */
        let asserts = asserts.unique().collect_vec();
        if asserts.is_empty() {
            return Ok(());
        }

        self.s.push(1)?;
        // Define all the constants
        for var in vars {
            log::trace!("Declaring constant {}", var);
            self.s.declare_const(var.to_string(), "Int")?;
        }

        // Define assumptions on constraints
        for assume in assumes {
            let sexp: SExp = assume.into();
            self.s.assert(format!("{}", sexp))?;
        }

        for fact in asserts {
            // XXX(rachit): Unnecessary clone
            if !self.check_fact(fact.clone())? {
                let mut err =
                    Error::malformed(format!("Cannot prove constraint {fact}"));
                for (msg, pos) in fact.notes() {
                    err = err.add_note(msg, *pos)
                }
                return Err(err);
            }
        }
        for share in sharing {
            let cons = SExp::from(share.clone());

            if !self.check_fact(cons)? {
                let mut err = Error::malformed(format!(
                    "Cannot prove constraint {share}"
                ));
                err = err.add_note("Event's delay must be longer than the difference between minimum start time and maximum end time of all other bindings.", share.event_bind.copy_span());
                for (msg, pos) in share.notes() {
                    err = err.add_note(msg, pos)
                }
                return Err(err);
            }
        }
        self.s.pop(1)?;

        Ok(())
    }

    fn check_fact(&mut self, fact: impl Into<SExp>) -> FilamentResult<bool> {
        let sexp = fact.into();
        self.s.push(1)?;
        let formula = format!("(not {})", sexp);
        log::trace!("Assert {}", formula);
        self.s.assert(formula)?;
        // Check that the assertion was unsatisfiable
        let unsat = !self.s.check_sat()?;
        if !unsat {
            log::trace!("MODEL: {:?}", self.s.get_model()?);
        }
        self.s.pop(1)?;
        Ok(unsat)
    }
}
