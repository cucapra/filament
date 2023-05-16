use super::Obligation;
use crate::ast::{self, Time, TimeSub};
use crate::diagnostics::{self, Diagnostics, InfoIdx};
use crate::errors::{Error, FilamentResult};
use itertools::Itertools;
use rsmt2::{SmtConf, Solver};

#[derive(Clone)]
/// Represents the sum of a time and a time sub
enum TimeDelSum {
    Time(Time),
    Sum(Time, TimeSub),
}

impl From<(Time, TimeSub)> for TimeDelSum {
    fn from((time, delay): (Time, TimeSub)) -> Self {
        match time.try_increment(delay) {
            Ok(t) => TimeDelSum::Time(t),
            Err((time, delay)) => TimeDelSum::Sum(time, delay),
        }
    }
}

impl std::fmt::Display for TimeDelSum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeDelSum::Time(t) => write!(f, "{}", t),
            TimeDelSum::Sum(t, d) => write!(f, "({}+{})", t, d),
        }
    }
}

impl From<TimeDelSum> for SExp {
    fn from(t: TimeDelSum) -> Self {
        match t {
            TimeDelSum::Time(t) => SExp::from(t),
            TimeDelSum::Sum(t, d) => {
                SExp(format!("(+ {} {})", SExp::from(t), SExp::from(d)))
            }
        }
    }
}

#[derive(Clone)]
pub struct ShareConstraint {
    /// Delay bounded by the share constraint
    event_bind: ast::Loc<ast::EventBind>,
    /// The events used to compute the minimum of start times
    starts: Vec<(Time, InfoIdx)>,
    /// The (event, delay) to compute the max of start times
    ends: Vec<(TimeDelSum, InfoIdx)>,
}

impl From<ast::Loc<ast::EventBind>> for ShareConstraint {
    fn from(event_bind: ast::Loc<ast::EventBind>) -> Self {
        Self {
            starts: vec![],
            ends: vec![],
            event_bind,
        }
    }
}

impl ShareConstraint {
    pub fn is_empty(&self) -> bool {
        self.starts.is_empty() && self.ends.is_empty()
    }

    pub fn add_bind_info(
        &mut self,
        start: ast::Loc<Time>,
        end: (Time, TimeSub),
        diag: &mut Diagnostics,
    ) {
        let td = TimeDelSum::from(end);
        let info = diag.add_info(
            format!("event use starts at `{start}' and ends at `{td}'"),
            start.pos(),
        );
        self.starts.push((start.take(), info));
        self.ends.push((td, info));
    }

    /// Transform the share constraint into an error
    fn error(self, diag: &mut diagnostics::Diagnostics) -> Error {
        let msg = format!("Cannot prove constraint {}", self);
        let mut err = Error::malformed(msg);
        err = err.add_note(diag.add_info(
            "event's delay must be longer than the difference between minimum start time and maximum end time of all invocations",
            self.event_bind.pos())
        );
        let all_notes = self
            .starts
            .iter()
            .map(|(_, i)| i)
            .chain(self.ends.iter().map(|(_, i)| i))
            .unique();
        err.notes.extend(all_notes);
        err
    }
}
impl std::fmt::Display for ShareConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let min = self.starts.iter().map(|(t, _)| t.to_string()).join(", ");
        let max = self.ends.iter().map(|(e, _)| format!("{e}")).join(", ");
        let delay = &self.event_bind.delay;
        write!(f, "{delay} >= max({max}) - min({min})")
    }
}
impl From<ShareConstraint> for SExp {
    fn from(mut sh: ShareConstraint) -> Self {
        let min = if sh.starts.len() > 1 {
            sh.starts
                .into_iter()
                .map(|(t, _)| SExp::from(t))
                .reduce(|a, b| SExp(format!("(min {} {})", a, b)))
                .unwrap()
        } else {
            SExp::from(sh.starts.pop().unwrap().0)
        };
        let max = if sh.ends.len() > 1 {
            sh.ends
                .into_iter()
                .map(|(t, _)| SExp::from(t))
                .reduce(|a, b| SExp(format!("(max {} {})", a, b)))
                .unwrap()
        } else {
            SExp::from(sh.ends.pop().unwrap().0)
        };

        SExp(format!(
            "(>= {} (- {max} {min}))",
            SExp::from(sh.event_bind.delay.inner().clone())
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

impl std::ops::Not for SExp {
    type Output = SExp;
    fn not(self) -> Self::Output {
        SExp(format!("(not {})", self.0))
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
    solver.declare_fun("pow2", &["Int"], "Int")?;
    solver.declare_fun("log2", &["Int"], "Int")?;
    Ok(())
}

/// An instance of a SMT solver for Filament constraints
pub struct FilSolver {
    /// Generate models for failing queries
    show_models: bool,
    /// Underlying solver
    s: Solver<()>,
}

impl FilSolver {
    pub fn new(show_models: bool) -> FilamentResult<Self> {
        let mut conf = SmtConf::default_z3();
        // Queries should time out about 30 seconds
        conf.option("-t:30000");

        let mut solver = conf.spawn(())?;
        solver.produce_models()?;
        solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

        define_prelude(&mut solver)?;
        Ok(Self {
            show_models,
            s: solver,
        })
    }

    pub fn declare_var(&mut self, var: ast::Id) {
        log::trace!("Declaring {}", var);
        self.s.declare_const(var.to_string(), "Int").unwrap();
        // All values must be positive
        self.s.assert(format!("(>= {} 0)", var)).unwrap();
    }

    pub fn assume(
        &mut self,
        assume: SExp,
        act_lit: Option<&rsmt2::actlit::Actlit>,
    ) {
        log::trace!("Assuming {}", assume);
        if let Some(act) = act_lit {
            self.s.assert_act(act, format!("{}", assume)).unwrap();
        } else {
            self.s.assert(format!("{}", assume)).unwrap();
        }
    }

    pub fn prove(
        &mut self,
        vars: impl IntoIterator<Item = ast::Id>,
        assumptions: Vec<SExp>,
        to_prove: Vec<Obligation>,
        sharing: Vec<ShareConstraint>,
        diag: &mut Diagnostics,
    ) {
        if to_prove.is_empty() {
            return;
        }

        let asserts = to_prove.into_iter().collect_vec();
        self.s.push(1).unwrap();
        // Define all the constants
        let vars = vars.into_iter().collect_vec();
        for var in &vars {
            self.declare_var(*var);
        }

        // Define assumptions on constraints
        for assume in assumptions {
            self.assume(assume, None);
        }

        for fact in asserts {
            if let Some(model) = self.check_fact(&fact, &vars) {
                let mut err = Error::from(fact);
                if self.show_models {
                    let info = diag.add_message(model);
                    err = err.add_note(info)
                }
                diag.add_error(err);
            }
        }
        for share in sharing {
            let obs = Obligation::new(SExp::from(share.clone()), "");
            if let Some(model) = self.check_fact(&obs, &vars) {
                let mut err = share.error(diag);
                if self.show_models {
                    let info = diag.add_message(model);
                    err = err.add_note(info)
                }
                diag.add_error(err);
            }
        }

        self.s.pop(1).unwrap();
    }

    /// Attempt to check facts.
    /// If the fact is false, add notes to the error showing the assignments that make it false.
    fn check_fact(
        &mut self,
        obl: &Obligation,
        vars: &[ast::Id],
    ) -> Option<String> {
        let mut requires_ctx = false;
        // If the fact requires defining vars, we need to push a context
        if !obl.defines.is_empty() {
            requires_ctx = true;
            self.s.push(1).unwrap();
            for v in &obl.defines {
                self.declare_var(*v);
            }
        }

        // Generate an activation literal
        let act = self.s.get_actlit().unwrap();

        let formula = format!("(not {})", obl.constraint());
        log::trace!("Assert {}", formula);
        self.s.assert_act(&act, formula.clone()).unwrap();
        // Check that the assertion was unsatisfiable
        let Some(sat) = self.s.check_sat_act_or_unk(Some(&act)).unwrap() else {
            return Some(format!("Query `{formula}' returned unknown. This likely happened because the query timed out."))
        };

        // If the assignment was not unsatisfiable, attempt to generate an assignment
        let assigns = if sat {
            log::trace!("MODEL: {:?}", self.s.get_model().unwrap());
            // If there are no relevant variables, we can't show a model
            let msg = if !vars.is_empty() {
                let assigns = self
                    .s
                    .get_values(
                        vars.iter()
                            .chain(obl.defines.iter())
                            .map(|n| n.to_string()),
                    )
                    .unwrap()
                    .into_iter()
                    .map(|(n, v)| format!("{}={}", n, v))
                    .collect_vec()
                    .join(", ");
                format!("assignment violates constraint: {}", assigns)
            } else {
                "".to_string()
            };
            Some(msg)
        } else {
            None
        };

        // Deactivate the activation literal
        self.s.de_actlit(act).unwrap();
        // Pop context if needed
        if requires_ctx {
            self.s.pop(1).unwrap();
        }
        assigns
    }
}
