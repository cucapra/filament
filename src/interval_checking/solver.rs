use rsmt2::{SmtConf, Solver};

use crate::{core, errors::FilamentResult};

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
    Ok(())
}

pub fn prove<'a, I, A, AV>(
    abstract_vars: AV,
    assumes: A,
    asserts: I,
) -> FilamentResult<Option<&'a core::Constraint<super::TimeRep>>>
where
    I: Iterator<Item = &'a core::Constraint<super::TimeRep>>,
    A: Iterator<Item = &'a core::Constraint<super::TimeRep>>,
    AV: Iterator<Item = &'a core::Id>,
{
    let parser = ();
    let mut conf = SmtConf::default_z3();

    // Immediately checks if the command to z3 succeeded.
    conf.check_success();

    let mut solver = conf.spawn(parser)?;
    solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

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
        if !check_fact(&mut solver, fact)? {
            return Ok(Some(fact));
        }
    }

    Ok(None)
}

fn check_fact<P>(
    solver: &mut Solver<P>,
    fact: &core::Constraint<super::TimeRep>,
) -> FilamentResult<bool> {
    let sexp = SExp::from(fact);
    log::info!("Assert (not {})", sexp);
    solver.push(1)?;
    solver.assert(format!("(not {})", sexp))?;
    // Check that the assertion was unsatisfiable
    let unsat = !solver.check_sat()?;
    solver.pop(1)?;
    Ok(unsat)
}
