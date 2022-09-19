use crate::errors::{Error, FilamentResult};
use crate::event_checker::ast;
use itertools::Itertools;
use rsmt2::{SmtConf, Solver};

pub struct ShareConstraints {
    /// The events used to compute the minimum of start times
    min: Vec<ast::TimeRep>,
    /// The (event, delay) to compute the max of start times
    max: Vec<(ast::TimeRep, ast::TimeSub)>,
    /// Delays bounded by this share constraint
    delays: Vec<ast::TimeSub>,
}
impl ShareConstraints {
    pub fn new(
        min: Vec<ast::TimeRep>,
        max: Vec<(ast::TimeRep, ast::TimeSub)>,
        delays: Vec<ast::TimeSub>,
    ) -> Self {
        Self { min, max, delays }
    }
}
impl std::fmt::Display for ShareConstraints {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let min = self.min.iter().map(|t| t.to_string()).join(", ");
        let max = self
            .max
            .iter()
            .map(|(t, d)| format!("{} + {}", t, d))
            .join(", ");
        let delays = self.delays.iter().map(|d| d.to_string()).join(", ");
        write!(f, "{delays} >= max({max}) - min({min})")
    }
}
impl From<&ShareConstraints> for Vec<SExp> {
    fn from(sh: &ShareConstraints) -> Self {
        let min = sh
            .min
            .iter()
            .map(|t| SExp::from(t))
            .reduce(|a, b| {
                SExp(format!("(min {} {})", SExp::from(a), SExp::from(b)))
            })
            .unwrap();
        let max = sh
            .max
            .iter()
            .map(|(t, d)| {
                SExp(format!("(+ {} {})", SExp::from(t), SExp::from(d)))
            })
            .reduce(|a, b| SExp(format!("(max {} {})", a, b)))
            .unwrap();
        sh.delays
            .iter()
            .map(|d| SExp(format!("(>= {} (- {max} {min}))", SExp::from(d))))
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
        // solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

        define_prelude(&mut solver)?;
        Ok(Self { s: solver })
    }

    pub fn prove<'a>(
        &mut self,
        abstract_vars: impl Iterator<Item = &'a ast::Id>,
        assumes: &[ast::Constraint],
        asserts: impl Iterator<Item = ast::Constraint>,
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

        // self.s.path_tee(std::path::PathBuf::from("./model.smt"))?;

        self.s.push(1)?;
        // Define all the constants
        for var in abstract_vars {
            log::trace!("Declaring constant {}", var);
            self.s.declare_const(var.to_string(), "Int")?;
        }

        // Define assumptions on constraints
        for assume in assumes {
            let sexp = SExp::from(assume);
            self.s.assert(format!("{}", sexp))?;
        }

        for fact in asserts {
            if !self.check_fact(&fact)? {
                let mut err =
                    Error::malformed(format!("Cannot prove constraint {fact}"));
                for (msg, pos) in fact.notes() {
                    err = err.add_note(msg, pos.clone())
                }
                return Err(err);
            }
        }
        for share in sharing {
            for sexp in Vec::<SExp>::from(&share) {
                if !self.check_fact(sexp.clone())? {
                    return Err(Error::malformed(format!(
                        "Cannot prove constraint {share}"
                    )));
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
