use rsmt2::{SmtConf, Solver};

use crate::{core, errors::FilamentResult};

/// A string that semantically represents an S-expression
pub struct SExp(pub String);
impl std::fmt::Display for SExp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn prove<'a, I, AV>(
    abstract_vars: AV,
    facts: I,
) -> FilamentResult<Option<&'a super::Fact>>
where
    I: Iterator<Item = &'a super::Fact>,
    AV: Iterator<Item = &'a core::Id>,
{
    let parser = ();
    let mut conf = SmtConf::default_z3();

    // Immediately checks if the command to z3 succeeded.
    conf.check_success();

    let mut solver = conf.spawn(parser)?;

    solver.path_tee(std::path::PathBuf::from("./model.smt"))?;

    // Define all the constants
    for var in abstract_vars {
        log::info!("Declaring constant {}", var);
        solver.declare_const(var.to_string(), "Int")?;
    }

    for fact in facts {
        if !check_fact(&mut solver, fact)? {
            return Ok(Some(fact));
        }
    }

    Ok(None)
}

fn check_fact<P>(
    solver: &mut Solver<P>,
    fact: &super::Fact,
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
