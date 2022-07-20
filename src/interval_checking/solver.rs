use crate::errors::{Error, FilamentResult};
use crate::event_checker::ast;
use itertools::Itertools;
use rsmt2::{SmtConf, Solver};

/// A string that semantically represents an S-expression
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
        "abs",
        &[("x", "Int")],
        "Int",
        "(ite (< x 0) (- x) x)",
    )?;
    Ok(())
}

pub fn prove<'a>(
    abstract_vars: impl Iterator<Item = &'a ast::Id>,
    assumes: &[ast::Constraint],
    asserts: impl Iterator<Item = ast::Constraint>,
) -> FilamentResult<()> {
    // Locally simplify as many asserts as possible
    let asserts = asserts
        .into_iter()
        .flat_map(|con| con.simplify())
        .collect_vec();

    let mut conf = SmtConf::default_z3();
    // Immediately checks if the command to z3 succeeded.
    conf.check_success();

    let mut solver = conf.spawn(())?;
    // solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

    define_prelude(&mut solver)?;

    // Define all the constants
    for var in abstract_vars {
        log::info!("Declaring constant {}", var);
        solver.declare_const(var.to_string(), "Int")?;
    }

    // Define assumptions on constraints
    for assume in assumes {
        let sexp = SExp::from(assume);
        solver.assert(format!("{}", sexp))?;
    }

    for fact in asserts {
        if !check_fact(&mut solver, &fact)? {
            let mut err =
                Error::malformed(format!("Cannot prove constraint {fact}"));
            for (msg, pos) in fact.notes() {
                err = err.add_note(msg, pos.clone())
            }
            return Err(err);
        }
    }

    Ok(())
}

fn check_fact<P>(
    solver: &mut Solver<P>,
    fact: impl Into<SExp>,
) -> FilamentResult<bool> {
    let sexp = fact.into();
    log::info!("Assert (not {})", sexp);
    solver.push(1)?;
    solver.assert(format!("(not {})", sexp))?;
    // Check that the assertion was unsatisfiable
    let unsat = !solver.check_sat()?;
    solver.pop(1)?;
    Ok(unsat)
}
