use fil_ast as ast;
use fil_ir::{self as ir, AddCtx, MutCtx};
use fil_utils::GPosIdx;
use std::path::PathBuf;

pub fn create_delay_register(ctx: &mut ir::Context) -> ir::CompIdx {
    let mut sv_file = <PathBuf as From<_>>::from(file!());
    sv_file.pop();
    sv_file.push("register.sv");

    log::debug!(
        "Adding external scheduling register from {}",
        sv_file.display()
    );

    // Next, build the external component
    let mut comp =
        ir::Component::new(ir::CompType::External, Default::default());

    let mut src_info =
        ir::InterfaceSrc::new("__SchedulingDelayRegister".into(), None);

    // Set up parameters to the component

    let width = comp.add(ir::Info::param("WIDTH".into(), GPosIdx::UNKNOWN));
    let width = comp.add(ir::Param::new(ir::ParamOwner::Sig, width));

    let delay = comp.add(ir::Info::param("DELAY".into(), GPosIdx::UNKNOWN));
    let delay = comp.add(ir::Param::new(ir::ParamOwner::Sig, delay));

    let live = comp.add(ir::Info::param("LIVE".into(), GPosIdx::UNKNOWN));
    let live = comp.add(ir::Param::new(ir::ParamOwner::Sig, live));

    // Add the parameters to the component
    src_info.params.push(width, "WIDTH".into());
    src_info.params.push(delay, "DELAY".into());
    src_info.params.push(live, "LIVE".into());

    // Intern the proposition that LIVE >= 1
    let live_expr = comp.add(ir::Expr::Param(live));
    let live_prop = ir::Prop::Cmp(ir::CmpOp::gte(
        live_expr,
        comp.add(ir::Expr::Concrete(1)),
    ));
    let live_prop = comp.add(live_prop);
    let live_info = comp.add(ir::Info::assert(ir::info::Reason::misc(
        "Signature assumption",
        GPosIdx::UNKNOWN,
    )));

    comp.add_param_assert([(live_prop, GPosIdx::UNKNOWN)]);
    comp.param_args = Box::new([width, delay, live]);

    let live_assumption = comp.assume(live_prop, live_info);
    comp.cmds.extend(live_assumption);

    let width = comp.add(ir::Expr::Param(width));
    let delay = comp.add(ir::Expr::Param(delay));
    let live = live_expr;

    // Set up the event of the component
    let event = comp.add(ir::Info::event(
        "G".into(),
        GPosIdx::UNKNOWN,
        GPosIdx::UNKNOWN,
        Some(("write_en".into(), GPosIdx::UNKNOWN)),
    ));
    let event = comp.add(ir::Event {
        delay: ir::TimeSub::Unit(live),
        info: event,
        has_interface: true,
    });
    src_info.events.push(event, "G".into());
    src_info.interface_ports.push(event, "write_en".into());
    comp.event_args = Box::new([event]);

    // Concrete expressions
    let zero = comp.add(ir::Expr::Concrete(0));
    let one = comp.add(ir::Expr::Concrete(1));
    let delay_1 = comp.add(ir::Expr::Bin {
        op: ast::Op::Add,
        lhs: delay,
        rhs: live,
    });
    // Set up ports to the component

    // clk and reset ports are unannotated
    comp.unannotated_ports =
        Box::new(vec![("clk".into(), 1), ("reset".into(), 1)]);

    // input port
    let input = ir::Port {
        owner: ir::PortOwner::Sig {
            dir: ir::Direction::Out,
        },
        width,
        live: ir::Liveness {
            idxs: vec![],
            lens: vec![comp.add(ir::Expr::Concrete(1))],
            range: ir::Range {
                start: comp.add(ir::Time {
                    event,
                    offset: zero,
                }),
                end: comp.add(ir::Time { event, offset: one }),
            },
        },
        info: comp.add(ir::Info::port(
            "in".into(),
            GPosIdx::UNKNOWN,
            GPosIdx::UNKNOWN,
            GPosIdx::UNKNOWN,
        )),
    };
    let input = comp.add(input);
    let input_param = ir::Param {
        owner: ir::ParamOwner::bundle(input),
        info: comp.add(ir::Info::param("_".into(), GPosIdx::UNKNOWN)),
    };
    let input_param = comp.add(input_param);

    comp.get_mut(input).live.idxs.push(input_param);
    src_info.ports.push(input, "in".into());

    // output port
    let output = ir::Port {
        owner: ir::PortOwner::Sig {
            dir: ir::Direction::In,
        },
        width,
        live: ir::Liveness {
            idxs: vec![],
            lens: vec![comp.add(ir::Expr::Concrete(1))],
            range: ir::Range {
                start: comp.add(ir::Time {
                    event,
                    offset: delay,
                }),
                end: comp.add(ir::Time {
                    event,
                    offset: delay_1,
                }),
            },
        },
        info: comp.add(ir::Info::port(
            "out".into(),
            GPosIdx::UNKNOWN,
            GPosIdx::UNKNOWN,
            GPosIdx::UNKNOWN,
        )),
    };
    let output = comp.add(output);

    let output_param = ir::Param {
        owner: ir::ParamOwner::bundle(output),
        info: comp.add(ir::Info::param("_".into(), GPosIdx::UNKNOWN)),
    };
    let output_param = comp.add(output_param);

    comp.get_mut(output).live.idxs.push(output_param);
    src_info.ports.push(output, "out".into());

    comp.src_info = Some(src_info);
    comp.port_attrs = ir::DenseIndexInfo::with_default(comp.ports().len());

    // Add the component to the context

    let compidx = ctx.add(comp);

    // Add the component to the external list
    let filename = sv_file.to_str().unwrap().to_string();
    ctx.externals.entry(filename).or_default().push(compidx);

    compidx
}
