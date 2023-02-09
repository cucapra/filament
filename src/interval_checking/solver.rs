use crate::core::{self, TimeRep};
use crate::errors::{Error, FilamentResult, WithPos};
use crate::utils::GPosIdx;
use itertools::Itertools;
use rsmt2::{SmtConf, Solver};

#[derive(Clone)]
pub struct ShareConstraints<T: TimeRep> {
    /// The events used to compute the minimum of start times
    min: Vec<T>,
    /// The (event, delay) to compute the max of start times
    max: Vec<(T, T::SubRep)>,
    /// Delays bounded by this share constraint
    delays: Vec<core::EventBind<T>>,
    /// Additional error information
    notes: Vec<(String, GPosIdx)>,
}

impl<T: TimeRep> Default for ShareConstraints<T> {
    fn default() -> Self {
        Self {
            min: vec![],
            max: vec![],
            delays: vec![],
            notes: vec![],
        }
    }
}

impl<T: TimeRep> ShareConstraints<T> {
    pub fn add_note<S: Into<String>>(&mut self, msg: S, pos: GPosIdx) {
        self.notes.push((msg.into(), pos));
    }

    pub fn notes(self) -> Vec<(String, GPosIdx)> {
        self.notes
    }

    pub fn add_bind_info(
        &mut self,
        start: T,
        end: (T, T::SubRep),
        pos: GPosIdx,
    ) {
        self.add_note(
            format!(
                "Invocation starts at `{start}' and finishes at `{}+{}'",
                end.0, end.1
            ),
            pos,
        );
        self.min.push(start);
        self.max.push(end);
    }

    pub fn add_delays(
        &mut self,
        delays: impl Iterator<Item = core::EventBind<T>>,
    ) {
        self.delays.extend(delays);
    }
}
impl<T: TimeRep> std::fmt::Display for ShareConstraints<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let min = self.min.iter().map(|t| t.to_string()).join(", ");
        let max = self
            .max
            .iter()
            .map(|(t, d)| format!("{} + {}", t, d))
            .join(", ");
        let delays = self.delays.iter().map(|d| d.delay.to_string()).join(", ");
        write!(f, "{delays} >= max({max}) - min({min})")
    }
}
impl<T: TimeRep> From<ShareConstraints<T>> for Vec<SExp> {
    fn from(sh: ShareConstraints<T>) -> Self {
        let min = sh
            .min
            .into_iter()
            .map(|t| t.into())
            .reduce(|a, b| SExp(format!("(min {} {})", a, b)))
            .unwrap();
        let max = sh
            .max
            .into_iter()
            .map(|(t, d)| SExp(format!("(+ {} {})", t.into(), d.into())))
            .reduce(|a, b| SExp(format!("(max {} {})", a, b)))
            .unwrap();
        sh.delays
            .into_iter()
            .map(|d| SExp(format!("(>= {} (- {max} {min}))", d.delay.into())))
            .collect()
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

pub struct FilSolver<T: TimeRep> {
    s: Solver<()>,
    _t: std::marker::PhantomData<T>,
}

impl<T: TimeRep> FilSolver<T> {
    pub fn new() -> FilamentResult<Self> {
        let mut conf = SmtConf::default_z3();
        // Immediately checks if the command to z3 succeeded.
        conf.check_success();

        let mut solver = conf.spawn(())?;
        // solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

        define_prelude(&mut solver)?;
        Ok(Self {
            s: solver,
            _t: std::marker::PhantomData,
        })
    }

    pub fn prove(
        &mut self,
        abstract_vars: impl Iterator<Item = core::Id>,
        assumes: Vec<core::Constraint<T>>,
        asserts: impl Iterator<Item = core::Constraint<T>>,
        sharing: Vec<ShareConstraints<T>>,
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

        // self.s.path_tee(std::path::PathBuf::from("./model.smt"))?;

        self.s.push(1)?;
        // Define all the constants
        for var in abstract_vars {
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
            // XXX(rachit): Unnecessary clone
            for (idx, sexp) in
                Vec::<SExp>::from(share.clone()).iter().enumerate()
            {
                if !self.check_fact(sexp.clone())? {
                    let mut err = Error::malformed(format!(
                        "Cannot prove constraint {share}"
                    ));
                    err = err.add_note("Event's delay must be longer than the difference between minimum start time and maximum end time of all other bindings.", share.delays[idx].copy_span());
                    for (msg, pos) in share.notes() {
                        err = err.add_note(msg, pos)
                    }
                    return Err(err);
                }
            }
        }
        self.s.pop(1)?;

        Ok(())
    }

    fn check_fact(&mut self, fact: impl Into<SExp>) -> FilamentResult<bool> {
        let sexp = fact.into();
        log::trace!("Assert (not {})", sexp);
        self.s.push(1)?;
        self.s.assert(format!("(not {})", sexp))?;
        // Check that the assertion was unsatisfiable
        let unsat = !self.s.check_sat()?;
        self.s.pop(1)?;
        Ok(unsat)
    }
}
