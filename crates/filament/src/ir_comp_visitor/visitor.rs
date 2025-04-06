use super::{Base, BaseComp, Underlying, UnderlyingComp};
use crate::ir_comp_visitor::IntoUdl;
use fil_ir::{self as ir, AddCtx, CompType, Ctx};
use fil_utils as utils;
use itertools::Itertools;

type SparseMap<T> = ir::SparseInfoMap<T, Base<T>, Underlying<T>>;

pub struct CompInfo {
    /// The base component
    pub comp: Base<ir::Component>,
    /// Signature ports
    pub ports: SparseMap<ir::Port>,
    /// Existential parameters
    pub exists: SparseMap<ir::Param>,
    /// Signature events
    pub events: SparseMap<ir::Event>,
}

/// Contains information passed to visitor functions.
/// This is re-generated when beginning to visit each component.
pub struct VisitorData<'comp> {
    /// The underlying component being visited
    pub underlying: UnderlyingComp<'comp>,
    /// The component being created
    pub base: BaseComp,
    /// Map of underlying parameters to new parameters
    pub param_map: SparseMap<ir::Param>,
    /// Map of underlying events to new events
    pub event_map: SparseMap<ir::Event>,
    /// Map of underlying ports to new ports
    pub port_map: SparseMap<ir::Port>,
    /// Map of underlying instances to new instances
    pub inst_map: SparseMap<ir::Instance>,
    /// Map of underlying invokes to new invokes
    pub inv_map: SparseMap<ir::Invoke>,
    /// Map of underlying components and their events/ports to new components
    comp_map:
        ir::SparseInfoMap<ir::Component, CompInfo, Underlying<ir::Component>>,
}

impl<'comp> VisitorData<'comp> {
    pub fn new(
        comp: &'comp ir::Component,
        typ: ir::CompType,
        attrs: utils::CompAttrs,
        comp_map: ir::SparseInfoMap<
            ir::Component,
            CompInfo,
            Underlying<ir::Component>,
        >,
    ) -> Self {
        Self {
            underlying: UnderlyingComp::new(comp),
            base: BaseComp::new(ir::Component::new(typ, attrs)),
            param_map: SparseMap::default(),
            event_map: SparseMap::default(),
            port_map: SparseMap::default(),
            inst_map: SparseMap::default(),
            inv_map: SparseMap::default(),
            comp_map,
        }
    }

    pub fn comp_info(&self, comp: Underlying<ir::Component>) -> &CompInfo {
        self.comp_map.get(comp)
    }
}

/// A visitor for the commands by detaching them from the underlying component.
pub trait Visitor
where
    Self: Sized,
{
    fn info(
        &mut self,
        info: Underlying<ir::Info>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Info> {
        let info = data.underlying.get(info);
        data.base.add(info.clone())
    }

    /// Definition of a parameter.
    fn param_def(
        &mut self,
        param: Underlying<ir::Param>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Param> {
        let ir::Param { owner, info } = data.underlying.get(param).clone();

        let param = ir::Param {
            owner: match owner {
                fil_ir::ParamOwner::Sig
                | ir::ParamOwner::Loop
                | ir::ParamOwner::Exists { .. } => owner,
                fil_ir::ParamOwner::Let { bind } => ir::ParamOwner::Let {
                    bind: bind.map(|b| self.expr(b.ul(), data).get()),
                },
                fil_ir::ParamOwner::Instance { inst, base } => {
                    let inst = *data.inst_map.get(inst.ul());

                    ir::ParamOwner::Instance {
                        inst: inst.get(),
                        base,
                    }
                }
                fil_ir::ParamOwner::Bundle(port) => {
                    ir::ParamOwner::Bundle(self.port_use(port.ul(), data).get())
                }
            },
            info: self.info(info.ul(), data).get(),
        };
        data.base.add(param)
    }

    /// Use of a parameter.
    fn param_use(
        &self,
        param: Underlying<ir::Param>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Param> {
        *data.param_map.get(param)
    }

    fn event_def(
        &mut self,
        event: Underlying<ir::Event>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Event> {
        let ir::Event {
            delay,
            info,
            has_interface,
        } = data.underlying.get(event).clone();

        let delay = self.timesub(delay, data);
        let info = self.info(info.ul(), data);

        data.base.add(ir::Event {
            delay,
            info: info.get(),
            has_interface,
        })
    }

    fn event_use(
        &self,
        event: Underlying<ir::Event>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Event> {
        *data.event_map.get(event)
    }

    fn timesub(
        &mut self,
        timesub: ir::TimeSub,
        data: &mut VisitorData<'_>,
    ) -> ir::TimeSub {
        match timesub {
            ir::TimeSub::Unit(e) => {
                let e = self.expr(e.ul(), data);

                ir::TimeSub::Unit(e.get())
            }
            ir::TimeSub::Sym { l, r } => {
                let l = self.time(l.ul(), data);
                let r = self.time(r.ul(), data);

                ir::TimeSub::Sym {
                    l: l.get(),
                    r: r.get(),
                }
            }
        }
    }

    fn time(
        &mut self,
        time: Underlying<ir::Time>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Time> {
        let ir::Time { event, offset } = *data.underlying.get(time);
        let event = self.event_use(event.ul(), data);
        let offset = self.expr(offset.ul(), data);

        let time = ir::Time {
            event: event.get(),
            offset: offset.get(),
        };
        data.base.add(time)
    }

    fn expr(
        &mut self,
        expr: Underlying<ir::Expr>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Expr> {
        match data.underlying.get(expr).clone() {
            ir::Expr::Param(idx) => {
                let param = self.param_use(idx.ul(), data);
                data.base.add(ir::Expr::Param(param.get()))
            }
            ir::Expr::Concrete(v) => data.base.num(v),
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(lhs.ul(), data);
                let rhs = self.expr(rhs.ul(), data);

                data.base.add(ir::Expr::Bin {
                    op,
                    lhs: lhs.get(),
                    rhs: rhs.get(),
                })
            }
            ir::Expr::Fn { op, args } => {
                let args = args
                    .into_iter()
                    .map(|e| self.expr(e.ul(), data).get())
                    .collect_vec();

                data.base.add(ir::Expr::Fn { op, args })
            }
            ir::Expr::If { cond, then, alt } => {
                let cond = self.prop(cond.ul(), data);
                let then = self.expr(then.ul(), data);
                let alt = self.expr(alt.ul(), data);

                data.base.add(ir::Expr::If {
                    cond: cond.get(),
                    then: then.get(),
                    alt: alt.get(),
                })
            }
        }
    }

    fn prop(
        &mut self,
        prop: Underlying<ir::Prop>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Prop> {
        match data.underlying.get(prop).clone() {
            ir::Prop::True => data.base.add(ir::Prop::True),
            ir::Prop::False => data.base.add(ir::Prop::False),
            ir::Prop::Cmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.expr(lhs.ul(), data);
                let rhs = self.expr(rhs.ul(), data);

                data.base.add(ir::Prop::Cmp(ir::CmpOp {
                    op,
                    lhs: lhs.get(),
                    rhs: rhs.get(),
                }))
            }
            ir::Prop::TimeCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.time(lhs.ul(), data);
                let rhs = self.time(rhs.ul(), data);

                data.base.add(ir::Prop::TimeCmp(ir::CmpOp {
                    op,
                    lhs: lhs.get(),
                    rhs: rhs.get(),
                }))
            }
            ir::Prop::TimeSubCmp(ir::CmpOp { op, lhs, rhs }) => {
                let lhs = self.timesub(lhs, data);
                let rhs = self.timesub(rhs, data);

                data.base
                    .add(ir::Prop::TimeSubCmp(ir::CmpOp { op, lhs, rhs }))
            }
            ir::Prop::Not(idx) => {
                let idx = self.prop(idx.ul(), data);
                data.base.add(ir::Prop::Not(idx.get()))
            }
            ir::Prop::And(l, r) => {
                let l = self.prop(l.ul(), data);
                let r = self.prop(r.ul(), data);

                data.base.add(ir::Prop::And(l.get(), r.get()))
            }
            ir::Prop::Or(l, r) => {
                let l = self.prop(l.ul(), data);
                let r = self.prop(r.ul(), data);

                data.base.add(ir::Prop::Or(l.get(), r.get()))
            }
            ir::Prop::Implies(l, r) => {
                let l = self.prop(l.ul(), data);
                let r = self.prop(r.ul(), data);

                data.base.add(ir::Prop::Implies(l.get(), r.get()))
            }
        }
    }

    fn port_def(
        &mut self,
        port: Underlying<ir::Port>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Port> {
        let ir::Port {
            owner,
            width,
            live: ir::Liveness { idxs, lens, range },
            info,
        } = data.underlying.get(port).clone();

        let owner = match owner {
            ir::PortOwner::Sig { dir } => ir::PortOwner::Sig { dir },
            ir::PortOwner::Inv { inv, dir, base } => {
                let inv = *data.inv_map.get(inv.ul());
                let (port, comp) = base.take();
                let base_info = data.comp_map.get(comp.ul());

                ir::PortOwner::Inv {
                    inv: inv.get(),
                    dir,
                    base: ir::Foreign::new(
                        base_info.ports.get(port.ul()).get(),
                        base_info.comp.get(),
                    ),
                }
            }
            ir::PortOwner::Local => ir::PortOwner::Local,
        };

        let port = ir::Port {
            owner,
            width: self.expr(width.ul(), data).get(),
            live: ir::Liveness {
                idxs: idxs
                    .into_iter()
                    .map(|idx| self.param_use(idx.ul(), data).get())
                    .collect_vec(),
                lens: lens
                    .into_iter()
                    .map(|lens| self.expr(lens.ul(), data).get())
                    .collect_vec(),
                range: ir::Range {
                    start: self.time(range.start.ul(), data).get(),
                    end: self.time(range.end.ul(), data).get(),
                },
            },
            info: self.info(info.ul(), data).get(),
        };

        data.base.add(port)
    }

    fn port_use(
        &mut self,
        port: Underlying<ir::Port>,
        data: &mut VisitorData<'_>,
    ) -> Base<ir::Port> {
        *data.port_map.get(port)
    }

    fn instance(
        &mut self,
        inst: Underlying<ir::Instance>,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::Instance {
            comp,
            args,
            lives,
            params,
            info,
        } = data.underlying.get(inst).clone();

        let inst = ir::Instance {
            comp: data.comp_map.get(comp.ul()).comp.get(),
            args: args
                .into_iter()
                .map(|e| self.expr(e.ul(), data).get())
                .collect_vec()
                .into_boxed_slice(),
            lives: lives
                .into_iter()
                .map(|ir::Range { start, end }| {
                    let start = self.time(start.ul(), data).get();
                    let end = self.time(end.ul(), data).get();
                    ir::Range { start, end }
                })
                .collect(),
            params: params
                .into_iter()
                .map(|p| self.param_def(p.ul(), data).get())
                .collect_vec(),
            info: self.info(info.ul(), data).get(),
        };

        vec![data.base.add(inst).get().into()]
    }

    fn invoke(
        &mut self,
        invoke: Underlying<ir::Invoke>,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::Invoke {
            inst,
            events,
            ports,
            info,
        } = data.underlying.get(invoke).clone();

        let inv = ir::Invoke {
            inst: data.inst_map.get(inst.ul()).get(),
            events: events
                .into_iter()
                .map(
                    |ir::EventBind {
                         delay,
                         arg,
                         info,
                         base,
                     }| {
                        let (evt, comp) = base.take();
                        let base_info = data.comp_map.get(comp.ul());
                        let base = ir::Foreign::new(
                            base_info.events.get(evt.ul()).get(),
                            base_info.comp.get(),
                        );

                        ir::EventBind {
                            delay: self.timesub(delay, data),
                            arg: self.time(arg.ul(), data).get(),
                            info: self.info(info.ul(), data).get(),
                            base,
                        }
                    },
                )
                .collect_vec(),
            ports: ports
                .into_iter()
                .map(|p| self.port_def(p.ul(), data).get())
                .collect_vec(),
            info,
        };

        vec![data.base.add(inv).get().into()]
    }

    fn bundle_def(
        &mut self,
        port: Underlying<ir::Port>,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        vec![ir::Command::BundleDef(self.port_def(port, data).get())]
    }

    fn access(
        &mut self,
        access: ir::Access,
        data: &mut VisitorData<'_>,
    ) -> ir::Access {
        let port = self.port_use(access.port.ul(), data).get();
        let ranges = access
            .ranges
            .into_iter()
            .map(|(start, end)| {
                let start = self.expr(start.ul(), data).get();
                let end = self.expr(end.ul(), data).get();
                (start, end)
            })
            .collect_vec();

        ir::Access { port, ranges }
    }

    fn connect(
        &mut self,
        connect: ir::Connect,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::Connect { src, dst, info } = connect;

        vec![
            ir::Connect {
                src: self.access(src, data),
                dst: self.access(dst, data),
                info: self.info(info.ul(), data).get(),
            }
            .into(),
        ]
    }

    fn let_(
        &mut self,
        l: ir::Let,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::Let { param, expr } = l;

        vec![
            ir::Let {
                param: self.param_def(param.ul(), data).get(),
                expr: expr.map(|e| self.expr(e.ul(), data).get()),
            }
            .into(),
        ]
    }

    fn forloop(
        &mut self,
        forloop: ir::Loop,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::Loop {
            index,
            start,
            end,
            body,
        } = forloop;

        vec![
            ir::Loop {
                index: self.param_def(index.ul(), data).get(),
                start: self.expr(start.ul(), data).get(),
                end: self.expr(end.ul(), data).get(),
                body: self.commands(body, data),
            }
            .into(),
        ]
    }

    fn if_(
        &mut self,
        if_: ir::If,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::If { cond, then, alt } = if_;

        vec![
            ir::If {
                cond: self.prop(cond.ul(), data).get(),
                then: self.commands(then, data),
                alt: self.commands(alt, data),
            }
            .into(),
        ]
    }

    fn fact(
        &mut self,
        fact: ir::Fact,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let prop = self.prop(fact.prop.ul(), data);
        let reason = self.info(fact.reason.ul(), data);

        if fact.is_assert() {
            data.base.assert(prop, reason)
        } else {
            data.base.assume(prop, reason)
        }
        .into_iter()
        .collect()
    }

    fn exists(
        &mut self,
        exists: ir::Exists,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let ir::Exists { param, expr } = exists;

        vec![
            ir::Exists {
                // param use instead of param def here as existentials are defined in the signature
                param: self.param_use(param.ul(), data).get(),
                expr: self.expr(expr.ul(), data).get(),
            }
            .into(),
        ]
    }

    fn commands(
        &mut self,
        cmds: Vec<ir::Command>,
        data: &mut VisitorData<'_>,
    ) -> Vec<ir::Command> {
        let mut new_cmds = Vec::new();
        for cmd in cmds {
            let cmds = match cmd {
                fil_ir::Command::Instance(idx) => self.instance(idx.ul(), data),
                fil_ir::Command::Invoke(idx) => self.invoke(idx.ul(), data),
                fil_ir::Command::BundleDef(idx) => {
                    self.bundle_def(idx.ul(), data)
                }
                fil_ir::Command::Connect(connect) => {
                    self.connect(connect, data)
                }
                fil_ir::Command::Let(l) => self.let_(l, data),
                fil_ir::Command::ForLoop(l) => self.forloop(l, data),
                fil_ir::Command::If(if_) => self.if_(if_, data),
                fil_ir::Command::Fact(fact) => self.fact(fact, data),
                fil_ir::Command::Exists(exists) => self.exists(exists, data),
            };

            new_cmds.extend(cmds);
        }

        new_cmds
    }

    fn sig(&mut self, data: &mut VisitorData<'_>) {
        let param_args =
            data.underlying.param_args().iter().copied().collect_vec();
        for param in param_args {
            self.param_def(param.ul(), data);
        }

        // Events can reference existential parameters, so we need to instantiate those here as well
        let exist_args = data.underlying.exist_params().collect_vec();
        for param in exist_args {
            self.param_def(param.ul(), data);

            // Next, add the associated exist assumes to the new component
            if let Some(assumes) = data.underlying.get_exist_assumes(param.ul())
            {
                let assumes = assumes
                    .into_iter()
                    .map(|p| self.prop(p.ul(), data))
                    .collect_vec();

                let base_param = self.param_use(param.ul(), data);

                data.base.add_exist_assumes(base_param, assumes);
            }
        }

        let event_args =
            data.underlying.event_args().iter().copied().collect_vec();
        for evt in event_args {
            self.event_def(evt.ul(), data);
        }

        // Deal with event and param asserts
        let event_asserts = data
            .underlying
            .get_event_asserts()
            .iter()
            .copied()
            .collect_vec();

        let event_asserts = event_asserts
            .into_iter()
            .map(|p| self.prop(p.ul(), data))
            .collect_vec();

        data.base.add_event_assert(event_asserts);

        let param_asserts = data
            .underlying
            .get_param_asserts()
            .iter()
            .copied()
            .collect_vec();

        let param_asserts = param_asserts
            .into_iter()
            .map(|p| self.prop(p.ul(), data))
            .collect_vec();

        data.base.add_param_assert(param_asserts);
    }

    fn data(
        comp: &ir::Component,
        comp_map: ir::SparseInfoMap<
            ir::Component,
            CompInfo,
            Underlying<ir::Component>,
        >,
    ) -> VisitorData<'_> {
        let typ = if comp.is_ext() {
            CompType::External
        } else if comp.is_gen() {
            CompType::Generated
        } else {
            CompType::Source
        };

        let attrs = comp.attrs.clone();

        VisitorData::new(comp, typ, attrs, comp_map)
    }

    fn do_pass(
        &mut self,
        comp: &ir::Component,
        comp_map: ir::SparseInfoMap<
            ir::Component,
            CompInfo,
            Underlying<ir::Component>,
        >,
    ) -> ir::Component {
        let mut data = Self::data(comp, comp_map);

        self.sig(&mut data);

        self.commands(data.underlying.cmds().clone(), &mut data);

        data.base.take()
    }
}
