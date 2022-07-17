use crate::{
    core,
    errors::{FilamentResult, WithPos},
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

fn transform_time(it: frontend::IntervalTime) -> core::FsmIdxs {
    assert!(
        type_check(&it) == EvType::Event,
        "interval time does not represent a valid event"
    );
    it.into()
}

fn transform_range(
    range: core::Range<frontend::IntervalTime>,
) -> core::Range<core::FsmIdxs> {
    let sp = range.copy_span();
    core::Range::new(range.start.into(), range.end.into()).set_span(sp)
}

fn transform_interval(
    interval: core::Interval<frontend::IntervalTime>,
) -> core::Interval<core::FsmIdxs> {
    let sp = interval.copy_span();
    core::Interval::new(
        transform_range(interval.within),
        interval.exact.map(transform_range),
    )
    .set_span(sp)
}

fn transform_control(
    con: core::Command<frontend::IntervalTime>,
) -> FilamentResult<core::Command<core::FsmIdxs>> {
    match con {
        core::Command::Invoke(inv) => {
            let pos = inv.copy_span();
            let abs: Vec<core::FsmIdxs> =
                inv.abstract_vars.into_iter().map(transform_time).collect();
            Ok(core::Invoke::new(inv.bind, inv.instance, abs, inv.ports)
                .set_span(pos)
                .into())
        }
        core::Command::Instance(ins) => Ok(core::Command::Instance(ins)),
        core::Command::Connect(con) => Ok(core::Command::Connect(con)),
        core::Command::Fsm(fsm) => Ok(core::Command::Fsm(fsm)),
    }
}

fn transform_port_def(
    pd: core::PortDef<frontend::IntervalTime>,
) -> core::PortDef<core::FsmIdxs> {
    let sp = pd.copy_span();
    core::PortDef::new(pd.name, transform_interval(pd.liveness), pd.bitwidth)
        .set_span(sp)
}

fn transform_interface_def(
    id: core::InterfaceDef<frontend::IntervalTime>,
) -> core::InterfaceDef<core::FsmIdxs> {
    let d = id.delay();
    let sp = id.copy_span();
    core::InterfaceDef::<core::FsmIdxs>::new(id.name, id.event, d).set_span(sp)
}

fn transform_constraints(
    con: core::Constraint<frontend::IntervalTime>,
) -> core::Constraint<core::FsmIdxs> {
    core::Constraint::new(
        transform_time(con.left),
        transform_time(con.right),
        con.op,
    )
}

fn transform_signature(
    sig: core::Signature<frontend::IntervalTime>,
) -> FilamentResult<core::Signature<core::FsmIdxs>> {
    let sig = core::Signature {
        inputs: sig.inputs.into_iter().map(transform_port_def).collect(),
        outputs: sig.outputs.into_iter().map(transform_port_def).collect(),
        constraints: sig
            .constraints
            .into_iter()
            .map(transform_constraints)
            .collect(),
        interface_signals: sig
            .interface_signals
            .into_iter()
            .map(transform_interface_def)
            .collect(),
        abstract_vars: sig.abstract_vars,
        name: sig.name,
        unannotated_ports: sig.unannotated_ports,
    };
    sig.validate()?;
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

            core::Component::new(transform_signature(comp.sig)?, commands)
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
