use crate::{
    core::{self, Component},
    errors::{Error, FilamentResult, WithPos},
    frontend,
};

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

fn transform_time(it: frontend::IntervalTime) -> FilamentResult<core::FsmIdxs> {
    if type_check(&it) != EvType::Event {
        Err(Error::malformed(format!("Not a well formed event: `{it}`")))
    } else {
        Ok(it.into())
    }
}

fn transform_range(
    range: core::Range<frontend::IntervalTime>,
) -> FilamentResult<core::Range<core::FsmIdxs>> {
    let sp = range.copy_span();

    transform_time(range.start)
        .and_then(|s| {
            transform_time(range.end)
                .map(|e| core::Range::new(s, e).set_span(sp.clone()))
        })
        .map_err(|e| e.add_note("Malformed interval", sp))
}

fn transform_interval(
    interval: core::Interval<frontend::IntervalTime>,
) -> FilamentResult<core::Interval<core::FsmIdxs>> {
    let sp = interval.copy_span();
    let ex = match interval.exact {
        Some(e) => Some(transform_range(e)?),
        None => None,
    };
    Ok(core::Interval::new(transform_range(interval.within)?, ex).set_span(sp))
}

fn transform_control(
    con: core::Command<frontend::IntervalTime>,
) -> FilamentResult<core::Command<core::FsmIdxs>> {
    match con {
        core::Command::Invoke(inv) => {
            let pos = inv.copy_span();
            let abs: Vec<core::FsmIdxs> = inv
                .abstract_vars
                .into_iter()
                .map(transform_time)
                .collect::<FilamentResult<_>>()?;
            Ok(core::Invoke::new(inv.bind, inv.instance, abs, inv.ports)
                .set_span(pos)
                .into())
        }
        core::Command::Instance(ins) => Ok(core::Command::Instance(ins)),
        core::Command::Connect(con) => Ok(core::Command::Connect(con)),
        core::Command::Fsm(fsm) => Ok(core::Command::Fsm(fsm)),
    }
}

fn transform_port_def<W: Clone>(
    pd: core::PortDef<frontend::IntervalTime, W>,
) -> FilamentResult<core::PortDef<core::FsmIdxs, W>> {
    let sp = pd.copy_span();
    Ok(core::PortDef::new(
        pd.name,
        transform_interval(pd.liveness)?,
        pd.bitwidth,
    )
    .set_span(sp))
}

fn transform_interface_def(
    id: core::InterfaceDef<frontend::IntervalTime>,
) -> FilamentResult<core::InterfaceDef<core::FsmIdxs>> {
    let sp = id.copy_span();
    let d = transform_time(id.end)
        .map_err(|e| e.add_note("Malformed interval", sp.clone()))?;
    Ok(core::InterfaceDef::<core::FsmIdxs>::new(
        id.name, id.event, d, id.phantom,
    )
    .set_span(sp))
}

fn transform_constraints(
    con: core::Constraint<frontend::IntervalTime>,
) -> FilamentResult<core::Constraint<core::FsmIdxs>> {
    con.map(&transform_time)
}

fn transform_signature<W: Clone>(
    sig: core::Signature<frontend::IntervalTime, W>,
) -> FilamentResult<core::Signature<core::FsmIdxs, W>> {
    let sig = core::Signature {
        params: sig.params,
        inputs: sig
            .inputs
            .into_iter()
            .map(transform_port_def)
            .collect::<FilamentResult<_>>()?,
        outputs: sig
            .outputs
            .into_iter()
            .map(transform_port_def)
            .collect::<FilamentResult<_>>()?,
        constraints: sig
            .constraints
            .into_iter()
            .map(transform_constraints)
            .collect::<FilamentResult<_>>()?,
        interface_signals: sig
            .interface_signals
            .into_iter()
            .map(transform_interface_def)
            .collect::<FilamentResult<_>>()?,
        abstract_vars: sig.abstract_vars,
        name: sig.name,
        unannotated_ports: sig.unannotated_ports,
    };
    Ok(sig)
}

pub fn check_and_transform(
    ns: core::Namespace<frontend::IntervalTime>,
) -> FilamentResult<core::Namespace<core::FsmIdxs>> {
    let components = ns
        .components
        .into_iter()
        .map(|comp| {
            let commands = comp
                .body
                .into_iter()
                .map(transform_control)
                .collect::<FilamentResult<Vec<_>>>()?;

            // Validate the component body
            Component::validate(&commands)?;

            Ok(core::Component::new(
                transform_signature(comp.sig)?,
                commands,
            ))
        })
        .collect::<FilamentResult<Vec<_>>>()?;

    Ok(core::Namespace {
        imports: ns.imports,
        externs: ns
            .externs
            .into_iter()
            .map(|(p, comps)| {
                Ok((
                    p,
                    comps
                        .into_iter()
                        .map(transform_signature)
                        .collect::<FilamentResult<_>>()?,
                ))
            })
            .collect::<FilamentResult<_>>()?,
        components,
    })
}
