use crate::{core, errors::FilamentResult};

/// The type ascribed to an interval time expression
#[derive(PartialEq)]
enum EvType {
    Event,
    Nat,
}

/// Checks that time specification follow the simple type system:
/// event   :: T
/// +       :: event -> nat -> event
/// max     :: event -> event -> event
fn type_check(it: &core::IntervalTime) -> FilamentResult<EvType> {
    match it {
        core::IntervalTime::Abstract(_) => Ok(EvType::Event),
        core::IntervalTime::Concrete(_) => Ok(EvType::Nat),
        core::IntervalTime::BinOp { op, left, right } => match op {
            core::TimeOp::Add => {
                match (type_check(left)?, type_check(right)?) {
                    (EvType::Event, EvType::Nat)
                    | (EvType::Nat, EvType::Event) => Ok(EvType::Event),
                    _ => panic!("Unexpected type for add expression"),
                }
            }
            core::TimeOp::Max => {
                match (type_check(left)?, type_check(right)?) {
                    (EvType::Event, EvType::Event) => Ok(EvType::Event),
                    _ => panic!("Unexpected type for max expression"),
                }
            }
        },
    }
}

fn check_control(con: &core::Command) -> FilamentResult<()> {
    match con {
        core::Command::Invoke(core::Invoke {
            rhs: core::Invocation { abstract_vars, .. },
            ..
        }) => abstract_vars.iter().try_for_each(|con| {
            type_check(con)?;
            Ok(())
        }),
        core::Command::When(core::When { commands, .. }) => {
            commands.iter().try_for_each(check_control)
        }
        core::Command::Instance(_) => Ok(()),
        core::Command::Connect(_) => Ok(()),
    }
}

pub fn check(namespace: &core::Namespace) -> FilamentResult<()> {
    for comp in &namespace.components {
        comp.body.iter().try_for_each(check_control)?
    }
    Ok(())
}
