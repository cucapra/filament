use fil_ir::{self as ir, AddCtx, Ctx, DisplayCtx, Foldable, MutCtx};
use fil_utils::{self as utils, GPosIdx};
use itertools::Itertools;

/// Sets the proper FSM Attributes for every component
pub struct Retime<'ctx, 'comp>
where
    'comp: 'ctx,
{
    ctx: &'ctx ir::Context,

    comp: &'comp mut ir::Component,

    /// Retiming register
    delay_register: ir::CompIdx,

    binding: ir::Bind<ir::ParamIdx, ir::ExprIdx>,
}

impl<'ctx, 'comp> Retime<'ctx, 'comp> {
    pub fn new(
        ctx: &'ctx ir::Context,
        comp: &'comp mut ir::Component,
        delay_register: ir::CompIdx,
    ) -> Self {
        Self {
            ctx,
            comp,
            delay_register,
            binding: Default::default(),
        }
    }

    pub fn comp(&mut self) {
        let cmds = std::mem::take(&mut self.comp.cmds);

        self.comp.cmds = cmds
            .into_iter()
            .flat_map(|cmd| match cmd {
                ir::Command::Connect(con) => self.connect(&con),
                ir::Command::Let(l) => {
                    self.let_(&l);
                    vec![l.into()]
                }
                _ => vec![cmd],
            })
            .collect();
    }

    fn event(&self) -> ir::Foreign<ir::Event, ir::Component> {
        ir::Foreign::new(
            self.ctx.get(self.delay_register).event_args[0],
            self.delay_register,
        )
    }

    fn let_(&mut self, l: &ir::Let) {
        let ir::Let { param, expr } = l;

        // add let-bound parameters to the binding for all expressions
        self.binding.push(*param, expr.unwrap());
    }

    fn connect(&mut self, con: &ir::Connect) -> Vec<ir::Command> {
        let ir::Connect {
            src:
                ir::Access {
                    port: src_port,
                    ranges: src_ranges,
                },
            dst:
                ir::Access {
                    port: dst_port,
                    ranges: dst_ranges,
                },
            ..
        } = con;

        let src_indices = utils::all_indices(
            src_ranges
                .iter()
                .map(|(s, e)| (s.concrete(self.comp), e.concrete(self.comp)))
                .collect_vec(),
        );

        let dst_indices = utils::all_indices(
            dst_ranges
                .iter()
                .map(|(s, e)| (s.concrete(self.comp), e.concrete(self.comp)))
                .collect_vec(),
        );

        let mut cmds = Vec::new();

        for (src_index, dst_index) in src_indices.into_iter().zip(dst_indices) {
            cmds.extend(
                self.invoke_register(
                    *src_port, *dst_port, src_index, dst_index,
                ),
            )
        }

        cmds
    }

    fn invoke_register(
        &mut self,
        srcidx: ir::PortIdx,
        dstidx: ir::PortIdx,
        src_index: Vec<u64>,
        dst_index: Vec<u64>,
    ) -> Vec<ir::Command> {
        let ir::Liveness {
            idxs: src_params,
            range: src_ranges,
            ..
        } = self.comp.get(srcidx).live.clone();
        let ir::Liveness {
            idxs: dst_params,
            range: dst_ranges,
            ..
        } = self.comp.get(dstidx).live.clone();

        let ir::Range {
            start: src_start,
            end: src_end,
        } = src_ranges;
        let ir::Range {
            start: dst_start,
            end: dst_end,
        } = dst_ranges;

        let width = self.comp.get(srcidx).width;

        let mut src_bind = ir::Bind::new(
            src_params
                .into_iter()
                .zip(src_index.iter().map(|&v| self.comp.num(v))),
        );
        let mut dst_bind = ir::Bind::new(
            dst_params
                .into_iter()
                .zip(dst_index.iter().map(|&v| self.comp.num(v))),
        );

        // Extend both with the global binding
        src_bind.extend(self.binding.iter().copied());
        dst_bind.extend(self.binding.iter().copied());

        // Substitute in the bindings
        let src_start = src_start
            .fold_with(self.comp, &mut |param| src_bind.get(&param).copied());
        let src_end = src_end
            .fold_with(self.comp, &mut |param| src_bind.get(&param).copied());
        let dst_start = dst_start
            .fold_with(self.comp, &mut |param| dst_bind.get(&param).copied());
        let dst_end = dst_end
            .fold_with(self.comp, &mut |param| dst_bind.get(&param).copied());

        let src_start = self.comp.get(src_start).offset.concrete(self.comp);
        let src_end = self.comp.get(src_end).offset.concrete(self.comp);
        let dst_start = self.comp.get(dst_start).offset.concrete(self.comp);
        let dst_end = self.comp.get(dst_end).offset.concrete(self.comp);

        assert!(
            dst_start >= src_start,
            "Port {} is connected to port {} but the destination port is scheduled before the source port",
            self.comp.display(srcidx),
            self.comp.display(dstidx)
        );

        // The range of the new connect
        // Because we want just one index in the bundle, the ranges should be
        // `[(a, a+1), (b, b+1), ...]``.
        let src_ranges = src_index
            .iter()
            .map(|&v| (self.comp.num(v), self.comp.num(v + 1)))
            .collect();
        let dst_ranges = dst_index
            .iter()
            .map(|&v| (self.comp.num(v), self.comp.num(v + 1)))
            .collect();

        // if dst_end happens after src_end, we need to insert a retiming register
        if dst_end > src_end {
            log::debug!(
                "Retiming register needed for {}: [{}, {}] -> {}: [{}, {}]",
                self.comp.display(srcidx),
                src_start,
                src_end,
                self.comp.display(dstidx),
                dst_start,
                dst_end,
            );
            let delay =
                self.comp.add(ir::Expr::Concrete(dst_start - src_start));

            let instname = format!(
                "retime_{}_{}_{}_{}_",
                self.comp.display(srcidx),
                src_index.into_iter().join("_"),
                self.comp.display(dstidx),
                dst_index.into_iter().join("_"),
            );

            let inst = ir::Instance {
                comp: self.delay_register,
                args: Box::new([
                    width,                                                  // WIDTH
                    delay, // DELAY
                    self.comp.add(ir::Expr::Concrete(dst_end - dst_start)), // LIVE
                ]),
                lives: Vec::default(),
                params: Vec::default(),
                info: self.comp.add(ir::Info::instance(
                    instname.clone().into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    vec![],
                )),
            };
            let inst = self.comp.add(inst);

            let event = self.comp.events().idx_iter().next().unwrap();

            // invoke happens right before src_end

            let invoke_time = ir::Time {
                event,
                offset: self.comp.add(ir::Expr::Concrete(src_end - 1)),
            };
            let invoke_time = self.comp.add(invoke_time);

            // Invoke the instance
            let inv = ir::Invoke {
                inst,
                events: vec![ir::EventBind {
                    arg: invoke_time,
                    delay: ir::TimeSub::Unit(delay),
                    info: self.comp.add(ir::Info::event_bind(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                    base: self.event(),
                }],
                ports: vec![], // will be filled in later
                info: self.comp.add(ir::Info::invoke(
                    format!("inv_{}", instname).into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    vec![GPosIdx::UNKNOWN],
                )),
            };
            let inv = self.comp.add(inv);

            // create the ports of the invoke
            let (inst_pidx, _) =
                self.ctx.get(self.delay_register).inputs().next().unwrap();

            let src_end_m_1 = self.comp.add(ir::Expr::Concrete(src_end - 1));
            let src_end = self.comp.add(ir::Expr::Concrete(src_end));
            let dst_start = self.comp.add(ir::Expr::Concrete(dst_start));
            let dst_end = self.comp.add(ir::Expr::Concrete(dst_end));

            let idx_param = ir::Param {
                // Temporary owner until we can create the invoke port
                owner: ir::ParamOwner::Bundle(ir::Idx::new(0)),
                info: self
                    .comp
                    .add(ir::Info::param("_".into(), GPosIdx::UNKNOWN)),
            };
            let idx_param = self.comp.add(idx_param);

            let inv_port_in = ir::Port {
                owner: ir::PortOwner::inv_in(
                    inv,
                    ir::Foreign::new(inst_pidx, self.delay_register),
                ),
                width,
                live: ir::Liveness {
                    idxs: vec![idx_param],
                    lens: vec![self.comp.add(ir::Expr::Concrete(1))],
                    range: ir::Range {
                        start: self.comp.add(ir::Time {
                            event,
                            offset: src_end_m_1,
                        }),
                        end: self.comp.add(ir::Time {
                            event,
                            offset: src_end,
                        }),
                    },
                },
                info: self.comp.add(ir::Info::port(
                    "in".into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                )),
            };

            let inv_port_in = self.comp.add(inv_port_in);

            self.comp
                .port_attrs
                .push(inv_port_in, utils::PortAttrs::default());

            self.comp.get_mut(idx_param).owner =
                ir::ParamOwner::Bundle(inv_port_in);

            let (inst_pidx, _) =
                self.ctx.get(self.delay_register).outputs().next().unwrap();

            let idx_param = ir::Param {
                owner: ir::ParamOwner::Bundle(ir::Idx::new(0)),
                info: self
                    .comp
                    .add(ir::Info::param("_".into(), GPosIdx::UNKNOWN)),
            };
            let idx_param = self.comp.add(idx_param);

            let inv_port_out = ir::Port {
                owner: ir::PortOwner::inv_out(
                    inv,
                    ir::Foreign::new(inst_pidx, self.delay_register),
                ),
                width,
                live: ir::Liveness {
                    idxs: vec![idx_param],
                    lens: vec![self.comp.add(ir::Expr::Concrete(1))],
                    range: ir::Range {
                        start: self.comp.add(ir::Time {
                            event,
                            offset: dst_start,
                        }),
                        end: self.comp.add(ir::Time {
                            event,
                            offset: dst_end,
                        }),
                    },
                },
                info: self.comp.add(ir::Info::port(
                    "out".into(),
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                    GPosIdx::UNKNOWN,
                )),
            };

            let inv_port_out = self.comp.add(inv_port_out);

            self.comp.get_mut(idx_param).owner =
                ir::ParamOwner::Bundle(inv_port_out);

            self.comp
                .port_attrs
                .push(inv_port_out, utils::PortAttrs::default());

            // Add the ports to the invoke

            let mut_inv = self.comp.get_mut(inv);
            mut_inv.ports = vec![inv_port_in, inv_port_out];

            // No bundles, all accesses have range (0, 1)
            let zero_one_range = vec![(
                self.comp.add(ir::Expr::Concrete(0)),
                self.comp.add(ir::Expr::Concrete(1)),
            )];

            // Add connections to the component
            vec![
                inst.into(),
                inv.into(),
                ir::Connect {
                    src: ir::Access {
                        port: srcidx,
                        ranges: src_ranges,
                    },
                    dst: ir::Access {
                        port: inv_port_in,
                        ranges: zero_one_range.clone(),
                    },
                    info: self.comp.add(ir::Info::connect(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                }
                .into(),
                ir::Connect {
                    src: ir::Access {
                        port: inv_port_out,
                        ranges: dst_ranges,
                    },
                    dst: ir::Access {
                        port: dstidx,
                        ranges: zero_one_range,
                    },
                    info: self.comp.add(ir::Info::connect(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                }
                .into(),
            ]
        } else {
            // No register necessary, just create the individual connect
            vec![
                ir::Connect {
                    src: ir::Access {
                        port: srcidx,
                        ranges: src_ranges,
                    },
                    dst: ir::Access {
                        port: dstidx,
                        ranges: dst_ranges,
                    },
                    info: self.comp.add(ir::Info::connect(
                        GPosIdx::UNKNOWN,
                        GPosIdx::UNKNOWN,
                    )),
                }
                .into(),
            ]
        }
    }
}
