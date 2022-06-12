use std::rc::Rc;
use itertools::Itertools;

use crate::frontend::ast as front;
use crate::{
    core::FsmIdxs,
    errors::{FilamentResult, WithPos},
    frontend,
};
use super::ast;

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
fn type_check(it: &frontend::IntervalTime) -> EvType {
    match it {
        frontend::IntervalTime::Abstract(_) => EvType::Event,
        frontend::IntervalTime::Concrete(_) => EvType::Nat,
        frontend::IntervalTime::Add { left, right } => {
            match (type_check(left), type_check(right)) {
                (EvType::Event, EvType::Nat) | (EvType::Nat, EvType::Event) => {
                    EvType::Event
                }
                _ => panic!("Unexpected type for add expression"),
            }
        }
        frontend::IntervalTime::Max { left, right } => {
            match (type_check(left), type_check(right)) {
                (EvType::Event, EvType::Event) => EvType::Event,
                _ => panic!("Unexpected type for max expression"),
            }
        }
    }
}

fn transform_time(it: frontend::IntervalTime) -> FsmIdxs {
    assert!(
        type_check(&it) == EvType::Event,
        "interval time does not represent a valid event"
    );
    it.into()
}

fn transform_range(
    range: front::Range,
) -> ast::Range {
    ast::Range {
        start: range.start.into(),
        end: range.end.into(),
    }
}

fn transform_interval(
    interval: front::Interval,
) -> ast::Interval {
    ast::Interval {
        within: transform_range(interval.within),
        exact: interval.exact.map(transform_range),
    }
}

fn transform_control(
    con: front::Command,
) -> FilamentResult<ast::Command> {
    match con {
        front::Command::Invoke(inv) => {
            let pos = inv.copy_span();
            let abs: Vec<FsmIdxs> =
                inv.abstract_vars.into_iter().map(transform_time).collect();
            Ok(ast::Invoke::new(inv.bind, inv.comp, abs, inv.ports)
                .set_span(pos)
                .into())
        }
        front::Command::Instance(ins) => Ok(ast::Command::Instance(ins)),
        front::Command::Connect(con) => Ok(ast::Command::Connect(con)),
        front::Command::Fsm(fsm) => Ok(ast::Command::Fsm(fsm)),
    }
}

fn transform_port_def(
    pd: front::PortDef,
) -> ast::PortDef {
    ast::PortDef {
        liveness: pd.liveness.map(transform_interval),
        name: pd.name,
        bitwidth: pd.bitwidth,
    }
}

fn transform_constraints(
    con: front::Constraint,
) -> ast::Constraint {
    ast::Constraint {
        left: transform_time(con.left),
        right: transform_time(con.right),
        op: con.op,
    }
}

fn transform_signature(
    sig: front::Signature,
) -> ast::Signature {
    ast::Signature {
        inputs: sig.inputs.into_iter().map(transform_port_def).collect(),
        outputs: sig.outputs.into_iter().map(transform_port_def).collect(),
        constraints: sig
            .constraints
            .into_iter()
            .map(transform_constraints)
            .collect(),
        name: sig.name,
        abstract_vars: sig.abstract_vars,
        interface_signals: sig
            .interface_signals
            .into_iter()
            .map(transform_port_def)
            .collect(),
    }
}

pub fn check_and_transform(
    ns: front::Namespace,
) -> FilamentResult<ast::Namespace> {
    let components = ns
        .components
        .into_iter()
        .map(|comp| {
            let commands = comp
                .body
                .into_iter()
                .map(transform_control)
                .collect::<FilamentResult<Vec<_>>>()?;

            Ok(ast::Component::new(
                transform_signature(Rc::try_unwrap(comp.sig).unwrap()),
                commands,
            ))
        })
        .collect::<FilamentResult<Vec<_>>>()?;

    Ok(ast::Namespace {
        imports: ns.imports,
        externs: ns
            .externs
            .into_iter()
            .map(|(p, comps)| {
                (p, comps.into_iter().map(transform_signature).collect())
            })
            .collect_vec(),
        components,
    })
}
