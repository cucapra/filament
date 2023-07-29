use std::collections::HashMap;

use itertools::Itertools;

use crate::ir::{
    Access, Bind, Command, CompIdx, Component, Connect, Context, Ctx,
    DenseIndexInfo, Expr, Foreign, Info, InvIdx, Invoke, Liveness, MutCtx,
    Port, PortIdx, PortOwner, Printer, Range, Subst, Time,
};

#[derive(Default)]
// Eliminates bundle ports by breaking them into multiple len-1 ports, and eliminates local ports altogether.
pub struct BundleElim {
    context: DenseIndexInfo<Component, HashMap<PortIdx, Vec<PortIdx>>>,
    local_map: HashMap<(PortIdx, usize), (PortIdx, usize)>,
}

impl BundleElim {
    /// Gets corresponding ports from the context given a component and a port access.
    fn get(
        &self,
        access: &Access,
        cidx: CompIdx,
        ctx: &Context,
    ) -> Vec<PortIdx> {
        let comp = ctx.get(cidx);

        let Access { port, start, end } = access;
        let start = start.as_concrete(comp).unwrap() as usize;
        let end = end.as_concrete(comp).unwrap() as usize;

        log::debug!("Getting port {}[{}..{}) from context.", port, start, end);

        (start..end)
            .map(|idx| {
                let mut group = (*port, idx);
                let (port, idx) = loop {
                    match self.local_map.get(&group) {
                        Some(&g) => group = g,
                        None => break group,
                    }
                };
                self.context[cidx][&port][idx]
            })
            .collect()
    }

    /// Compiles a port by breaking it into multiple len-1 ports.
    fn port(&self, pidx: PortIdx, comp: &mut Component) -> Vec<PortIdx> {
        let one = comp.add(Expr::Concrete(1));
        let Port {
            owner, width, live, ..
        } = comp.get(pidx).clone();

        let Liveness { idx, len, range } = live;

        let start = comp.get(range.start).clone();
        let end = comp.get(range.end).clone();

        let len = len.as_concrete(comp).unwrap();

        // if we need to preserve external interface information, we can't have bundle ports in the signature.
        if comp.src_info.is_some() && matches!(owner, PortOwner::Sig { .. }) {
            assert!(
                len == 1,
                "Bundle ports in the signature are not supported when external interface must be preserved."
            );

            // need to preserve the original portidx here to save the source information.
            return vec![pidx];
        }

        // creates an empty info struct for these new ports
        let info = comp.add(Info::empty());

        // create a single port for each element in the bundle.
        let ports = (0..len)
            .map(|i| {
                // binds the index parameter to the current bundle index
                let binding: Bind<_, _> =
                    Bind::new([(idx, comp.add(Expr::Concrete(i)))]);

                // calculates the offsets based on this binding and generates new start and end times.
                let offset = Subst::new(start.offset, &binding).apply(comp);
                let start = comp.add(Time {
                    event: start.event,
                    offset,
                });

                let offset = Subst::new(end.offset, &binding).apply(comp);
                let end = comp.add(Time {
                    event: end.event,
                    offset,
                });

                // creates a new liveness with the new start and end times and length one
                let live = Liveness {
                    idx, // this should technically be some null parameter, as it will refer to a deleted parameter now.
                    len: one,
                    range: Range { start, end },
                };

                // creates a new portowner based on the original owner
                let owner = match &owner {
                    PortOwner::Sig { dir } => {
                        PortOwner::Sig { dir: dir.clone() }
                    }
                    PortOwner::Inv { inv, dir, base } => {
                        let (key, owner) = base.take();
                        PortOwner::Inv {
                            inv: *inv,
                            dir: dir.clone(),
                            base: Foreign::new(
                                // maps the foreign to the corresponding single port
                                // this works because all signature ports are compiled first.
                                self.context[owner][&key][i as usize],
                                owner,
                            ),
                        }
                    }
                    PortOwner::Local => PortOwner::Local,
                };

                // adds the new port to the component and return its index
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
        // delete the corresponding parameter
        comp.delete(idx);
        ports
    }

    /// Compiles the signature of a component and adds the new ports to the context mapping.
    fn sig(&self, comp: &mut Component) -> HashMap<PortIdx, Vec<PortIdx>> {
        // loop through signature ports and compile them
        comp.ports()
            .idx_iter()
            .filter_map(|idx| {
                comp.get(idx).is_sig().then(|| (idx, self.port(idx, comp)))
            })
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
        // first take all the old ports and split them up
        let mappings = std::mem::take(ports)
            .into_iter()
            .map(|p| (p, self.port(p, comp)))
            .collect_vec();

        // add them back to the invoke (need to get mutably again because comp is mutated above)
        comp.get_mut(idx).ports =
            mappings.iter().flat_map(|(_, v)| v).copied().collect();

        mappings.into_iter().collect()
    }

    /// Compiles a connect command by breaking it into multiple simple connect commands
    /// Also eliminates local ports by storing their source bindings in the pass.
    fn connect(
        &self,
        connect: &Connect,
        cidx: CompIdx,
        ctx: &mut Context,
    ) -> Vec<Command> {
        let Connect { src, dst, .. } = connect;

        if !self.context.get(cidx).contains_key(&dst.port) {
            // we are writing to a local port here.
            return vec![];
        }

        // get the list of ports associated with each access in the connect.
        let src = self.get(src, cidx, ctx);
        let dst = self.get(dst, cidx, ctx);

        let comp = ctx.get_mut(cidx);
        assert!(
            src.len() == dst.len(),
            "Mismatched access lengths for connect `{}`",
            Printer::new(comp).connect_str(connect)
        );

        // split this single connects into `n` separate connects each with individual ports.
        // the local mapping optimization here works because it assumes that connects assigning to the local port
        // are defined before connects accessing the local port (I.E. assignments are in proper order).
        src.into_iter()
            .zip(dst.into_iter())
            .map(|(src, dst)| {
                Command::Connect(Connect {
                    src: Access::port(src, comp),
                    dst: Access::port(dst, comp),
                    info: comp.add(Info::empty()),
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

        self.local_map = comp
            .cmds
            .iter()
            .filter_map(|cmd| {
                let Command::Connect(con) = cmd else { return None };
                let Connect { src, dst, .. } = con;

                // need to check validity here because already deleted ports are still in the connect commands
                if comp.ports().is_valid(dst.port)
                    && comp.get(dst.port).is_local()
                {
                    let dst_start =
                        dst.start.as_concrete(comp).unwrap() as usize;
                    let dst_end = dst.end.as_concrete(comp).unwrap() as usize;
                    let src_start =
                        src.start.as_concrete(comp).unwrap() as usize;
                    let src_end = src.end.as_concrete(comp).unwrap() as usize;
                    assert!(
                        dst_end - dst_start == src_end - src_start,
                        "Mismatched access lengths for connect `{}`",
                        Printer::new(comp).connect_str(con)
                    );

                    Some(
                        (dst_start..dst_end)
                            .zip(src_start..src_end)
                            .map(|(d, s)| ((dst.port, d), (src.port, s))),
                    )
                } else {
                    None
                }
            })
            .flatten()
            .collect();

        log::debug!("Local Map: {:?}", self.local_map);

        comp.ports()
            .iter()
            .filter(|(_, port)| port.is_local())
            .map(|(idx, _)| idx)
            .collect_vec() // collect here to remove ownership problems
            .into_iter()
            .for_each(|idx| comp.delete(idx)); // invalidate these ports

        let cmds = comp
            .cmds
            .drain(..)
            .collect_vec() // collect here to avoid ownership issues
            .into_iter()
            .flat_map(|cmd| match cmd {
                Command::Connect(con) => {
                    self.connect(&con, cidx, ctx) // compile connect into simple connects
                }
                Command::Instance(_)
                | Command::Invoke(_)
                | Command::Fact(_) => vec![cmd], // keeps these commands unchanged
                Command::ForLoop(_) => {
                    unreachable!("For Loops should have been compiled away.")
                }
                Command::If(_) => {
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
        // compiles signature ports and adds them to the context
        for (idx, c) in ctx.comps.iter_mut() {
            visitor.context.push(idx, visitor.sig(c));
        }

        // second pass to compile all the other ports and actually split the connects.
        for idx in ctx.comps.idx_iter() {
            visitor.comp(idx, ctx);
        }
    }
}
