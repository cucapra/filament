use std::collections::HashMap;

use itertools::Itertools;

use crate::ir::{
    Access, Command, CompIdx, Component, Connect, Context, Ctx, DenseIndexInfo,
    Expr, Foreign, InvIdx, Invoke, Liveness, MutCtx, Port, PortIdx, Range,
    Time,
};

#[derive(Default)]
pub struct BundleElim {
    context: DenseIndexInfo<Component, HashMap<PortIdx, Vec<PortIdx>>>,
}

impl BundleElim {
    fn port(&self, pidx: PortIdx, comp: &mut Component) -> Vec<PortIdx> {
        let one = comp.add(Expr::Concrete(1));
        let Port {
            owner,
            width,
            live,
            info,
        } = comp.get(pidx).clone();

        let Liveness { idx, len, range } = live;

        let start = comp.get(range.start);
        let end = comp.get(range.end);

        assert!(
            start.event == end.event,
            "Range `{range}` cannot be represented as a simple offset"
        );

        let event = start.event;

        let start = start.offset;
        let end = end.offset;

        let len = len.as_concrete(comp).unwrap();

        let ports = (0..len)
            .map(|i| {
                let binding: HashMap<_, _> = [(idx, i)].into();

                let offset = start.resolve(comp, &binding);
                let start = comp.add(Time { event, offset });

                let offset = end.resolve(comp, &binding);
                let end = comp.add(Time { event, offset });
                let live = Liveness {
                    idx,
                    len: one,
                    range: Range { start, end },
                };

                let owner = match &owner {
                    crate::ir::PortOwner::Sig { dir } => {
                        crate::ir::PortOwner::Sig { dir: dir.clone() }
                    }
                    crate::ir::PortOwner::Inv { inv, dir, base } => {
                        crate::ir::PortOwner::Inv {
                            inv: *inv,
                            dir: dir.clone(),
                            base: Foreign::new(
                                self.context
                                    .get(base.owner)
                                    .get(&base.key)
                                    .unwrap()[i as usize],
                                base.owner,
                            ),
                        }
                    }

                    crate::ir::PortOwner::Local => crate::ir::PortOwner::Local,
                };

                comp.add(Port {
                    live,
                    owner,
                    info,
                    width,
                })
            })
            .collect();
        // delete the original port
        comp.delete(pidx);
        ports
    }

    /// Compiles the signature of a component and adds the new ports to the context mapping.
    fn sig(&self, comp: &mut Component) -> HashMap<PortIdx, Vec<PortIdx>> {
        // need to collect here because of ownership issues.
        comp.ports()
            .idx_iter()
            .filter(|p| comp.get(*p).is_sig())
            .collect_vec() // collect here to avoid ownership issues
            .into_iter()
            .map(|idx| (idx, self.port(idx, comp)))
            .collect()
    }

    /// Compiles the ports defined by an invocation and adds the new ports to the context mapping.
    /// Mutates the invocation in place, redefining its defined ports.
    fn inv(
        &self,
        idx: InvIdx,
        comp: &mut Component,
    ) -> HashMap<PortIdx, Vec<PortIdx>> {
        let Invoke { ports, .. } = comp.get_mut(idx);
        let mappings = ports.drain(..).collect_vec();
        let mappings = mappings
            .into_iter()
            .map(|p| (p, self.port(p, comp)))
            .collect_vec();

        comp.get_mut(idx)
            .ports
            .extend(mappings.iter().flat_map(|(_, v)| v));

        mappings.into_iter().collect()
    }

    fn connect(
        &self,
        connect: &Connect,
        cidx: CompIdx,
        ctx: &mut Context,
    ) -> Vec<Command> {
        let comp = ctx.get_mut(cidx);
        let mapping = self.context.get(cidx);

        let Connect { src, dst, info } = connect;

        let start = src.start.as_concrete(comp).unwrap() as usize;
        let end = src.end.as_concrete(comp).unwrap() as usize;
        let src = mapping.get(&src.port).unwrap()[start..end].to_vec();

        let start = dst.start.as_concrete(comp).unwrap() as usize;
        let end = dst.end.as_concrete(comp).unwrap() as usize;
        let dst = mapping.get(&dst.port).unwrap()[start..end].to_vec();

        assert!(
            src.len() == dst.len(),
            "Mismatched access lengths for connect `{}`",
            connect
        );

        src.into_iter()
            .zip(dst.into_iter())
            .map(|(src, dst)| {
                (Access::port(src, comp), Access::port(dst, comp))
            })
            .map(|(src, dst)| {
                Command::Connect(Connect {
                    src,
                    dst,
                    info: *info,
                })
            })
            .collect()
    }

    /// Compiles the body of a component and replaces all ports with their expanded versions.
    fn comp(&mut self, cidx: CompIdx, ctx: &mut Context) {
        let comp = ctx.get_mut(cidx);
        // compile invocations
        for idx in comp.invocations().idx_iter() {
            let pl = self.inv(idx, comp);
            self.context.get_mut(cidx).extend(pl);
        }

        // compile local ports
        for idx in comp
            .ports()
            .iter()
            .filter(|(_, port)| port.is_local())
            .map(|(idx, _)| idx)
            .collect_vec()
        {
            let pl = self.port(idx, comp);
            self.context.get_mut(cidx).insert(idx, pl);
        }

        let cmds = comp
            .cmds
            .drain(..)
            .collect_vec() // collect here to avoid ownership issues
            .into_iter()
            .flat_map(|cmd| match cmd {
                crate::ir::Command::Connect(con) => {
                    self.connect(&con, cidx, ctx)
                }
                crate::ir::Command::Instance(_)
                | crate::ir::Command::Invoke(_)
                | crate::ir::Command::Fact(_) => vec![cmd],
                crate::ir::Command::ForLoop(_) => {
                    unreachable!("For Loops should have been compiled away.")
                }
                crate::ir::Command::If(_) => {
                    unreachable!("Ifs should have been compiled away.")
                }
            })
            .collect_vec();

        // need to get here again because component is mutated above.
        let comp = ctx.get_mut(cidx);
        comp.cmds = cmds;
    }

    pub fn do_pass(ctx: &mut Context) {
        let mut visitor = Self::default();
        for (idx, c) in ctx.comps.iter_mut() {
            visitor.context.push(idx, visitor.sig(c));
        }

        for idx in ctx.comps.idx_iter() {
            visitor.comp(idx, ctx);
        }
    }
}
