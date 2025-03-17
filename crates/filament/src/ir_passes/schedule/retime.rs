use fil_ast as ast;
use fil_ir::{self as ir, AddCtx, Ctx, DisplayCtx, MutCtx};
use fil_utils::GPosIdx;
use std::path::PathBuf;

use crate::ir_visitor::{Action, Construct, Visitor};

/// Sets the proper FSM Attributes for every component
pub struct Retime {
    /// Retiming register
    delay_register: ir::CompIdx,
}

impl Retime {
    pub fn create_delay_register(ctx: &mut ir::Context) -> ir::CompIdx {
        let mut sv_file = <PathBuf as From<_>>::from(file!());
        sv_file.pop();
        sv_file.push("schedule.sv");

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

        comp.add_param_assert([live_prop]);
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

    pub fn event(
        &self,
        ctx: &ir::Context,
    ) -> ir::Foreign<ir::Event, ir::Component> {
        ir::Foreign::new(
            ctx.get(self.delay_register).event_args[0],
            self.delay_register,
        )
    }
}

impl Construct for Retime {
    fn from(_: &crate::cmdline::Opts, ctx: &mut fil_ir::Context) -> Self {
        let delay_register = Self::create_delay_register(ctx);

        Self { delay_register }
    }

    fn clear_data(&mut self) {}
}

impl Visitor for Retime {
    fn name() -> &'static str {
        "schedule-retime"
    }

    fn connect(
        &mut self,
        con: &mut fil_ir::Connect,
        data: &mut crate::ir_visitor::VisitorData,
    ) -> crate::ir_visitor::Action {
        let ir::Connect { src, dst, .. } = con;

        // Make sure there are no bundles
        if !(src.is_port(&data.comp)
            && dst.is_port(&data.comp)
            && src.port.is_not_bundle(&data.comp)
            && dst.port.is_not_bundle(&data.comp))
        {
            unreachable!(
                "Port {} and {} are bundles. Bundles are not supported in the scheduling pass. Please run bundle-elim first.",
                data.comp.display(src.port),
                data.comp.display(dst.port)
            );
        }

        let srcidx = src.port;
        let dstidx = dst.port;

        let src = data.comp.get(src.port);
        let dst = data.comp.get(dst.port);

        let ir::Range {
            start: src_start,
            end: src_end,
        } = src.live.range;
        let ir::Range {
            start: dst_start,
            end: dst_end,
        } = dst.live.range;

        let src_start = data.comp.get(src_start).offset.concrete(&data.comp);
        let src_end = data.comp.get(src_end).offset.concrete(&data.comp);
        let dst_start = data.comp.get(dst_start).offset.concrete(&data.comp);
        let dst_end = data.comp.get(dst_end).offset.concrete(&data.comp);

        assert!(
            dst_start >= src_start,
            "Port {} is connected to port {} but the destination port is scheduled before the source port",
            data.comp.display(srcidx),
            data.comp.display(dstidx)
        );

        // if dst_end happens after src_end, we need to insert a retiming register
        if dst_end > src_end {
            log::debug!(
                "Retiming register needed for {}: [{}, {}] -> {}: [{}, {}]",
                data.comp.display(srcidx),
                src_start,
                src_end,
                data.comp.display(dstidx),
                dst_start,
                dst_end,
            );
            let width = src.width;
            let delay =
                data.comp.add(ir::Expr::Concrete(dst_start - src_start));

            let instname = format!("retime_{}_{}", srcidx.get(), dstidx.get());

            let inst = ir::Instance {
                comp: self.delay_register,
                args: Box::new([
                    width,                                                  // WIDTH
                    delay, // DELAY
                    data.comp.add(ir::Expr::Concrete(dst_end - dst_start)), // LIVE
                ]),
                lives: Vec::default(),
                params: Vec::default(),
                info: data.comp.add(ir::Info::instance(
                    instname.clone().into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    vec![],
                )),
            };
            let inst = data.comp.add(inst);

            let event = data.comp.events().idx_iter().next().unwrap();

            // invoke happens right before src_end

            let invoke_time = ir::Time {
                event,
                offset: data.comp.add(ir::Expr::Concrete(src_end - 1)),
            };
            let invoke_time = data.comp.add(invoke_time);

            // Invoke the instance
            let inv = ir::Invoke {
                inst,
                events: vec![ir::EventBind {
                    arg: invoke_time,
                    delay: ir::TimeSub::Unit(delay),
                    info: data.comp.add(ir::Info::event_bind(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                    base: self.event(data.ctx()),
                }],
                ports: vec![], // will be filled in later
                info: data.comp.add(ir::Info::invoke(
                    format!("inv_{}", instname).into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    vec![GPosIdx::UNKNOWN],
                )),
            };
            let inv = data.comp.add(inv);

            // create the ports of the invoke
            let (inst_pidx, inst_param) =
                data.ctx().get(self.delay_register).inputs().next().unwrap();

            let inst_param = inst_param.live.idxs[0];

            let src_end_m_1 = data.comp.add(ir::Expr::Concrete(src_end - 1));
            let src_end = data.comp.add(ir::Expr::Concrete(src_end));
            let dst_start = data.comp.add(ir::Expr::Concrete(dst_start));
            let dst_end = data.comp.add(ir::Expr::Concrete(dst_end));

            let idx_param = ir::Param {
                owner: ir::ParamOwner::Instance {
                    inst,
                    base: ir::Foreign::new(inst_param, self.delay_register),
                },
                info: data
                    .comp
                    .add(ir::Info::param("_".into(), GPosIdx::UNKNOWN)),
            };
            let idx_param = data.comp.add(idx_param);

            let inv_port_in = ir::Port {
                owner: ir::PortOwner::inv_in(
                    inv,
                    ir::Foreign::new(inst_pidx, self.delay_register),
                ),
                width,
                live: ir::Liveness {
                    idxs: vec![idx_param],
                    lens: vec![data.comp.add(ir::Expr::Concrete(1))],
                    range: ir::Range {
                        start: data.comp.add(ir::Time {
                            event,
                            offset: src_end_m_1,
                        }),
                        end: data.comp.add(ir::Time {
                            event,
                            offset: src_end,
                        }),
                    },
                },
                info: data.comp.add(ir::Info::port(
                    "in".into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                )),
            };

            let inv_port_in = data.comp.add(inv_port_in);

            let (inst_pidx, inst_param) = data
                .ctx()
                .get(self.delay_register)
                .outputs()
                .next()
                .unwrap();

            let inst_param = inst_param.live.idxs[0];

            let idx_param = ir::Param {
                owner: ir::ParamOwner::Instance {
                    inst,
                    base: ir::Foreign::new(inst_param, self.delay_register),
                },
                info: data
                    .comp
                    .add(ir::Info::param("_".into(), GPosIdx::UNKNOWN)),
            };
            let idx_param = data.comp.add(idx_param);

            let inv_port_out = ir::Port {
                owner: ir::PortOwner::inv_out(
                    inv,
                    ir::Foreign::new(inst_pidx, self.delay_register),
                ),
                width,
                live: ir::Liveness {
                    idxs: vec![idx_param],
                    lens: vec![data.comp.add(ir::Expr::Concrete(1))],
                    range: ir::Range {
                        start: data.comp.add(ir::Time {
                            event,
                            offset: dst_start,
                        }),
                        end: data.comp.add(ir::Time {
                            event,
                            offset: dst_end,
                        }),
                    },
                },
                info: data.comp.add(ir::Info::port(
                    "out".into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                )),
            };

            let inv_port_out = data.comp.add(inv_port_out);

            // Add the ports to the invoke

            let mut_inv = data.comp.get_mut(inv);
            mut_inv.ports = vec![inv_port_in, inv_port_out];

            // No bundles, all accesses have range (0, 1)
            let ranges = vec![(
                data.comp.add(ir::Expr::Concrete(0)),
                data.comp.add(ir::Expr::Concrete(1)),
            )];

            // Add connections to the component
            Action::Change(vec![
                inst.into(),
                inv.into(),
                ir::Connect {
                    src: ir::Access {
                        port: srcidx,
                        ranges: ranges.clone(),
                    },
                    dst: ir::Access {
                        port: inv_port_in,
                        ranges: ranges.clone(),
                    },
                    info: data.comp.add(ir::Info::connect(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                }
                .into(),
                ir::Connect {
                    src: ir::Access {
                        port: inv_port_out,
                        ranges: ranges.clone(),
                    },
                    dst: ir::Access {
                        port: dstidx,
                        ranges,
                    },
                    info: data.comp.add(ir::Info::connect(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                }
                .into(),
            ])
        } else {
            Action::Continue
        }
    }
}
