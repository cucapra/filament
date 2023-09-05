use crate::{
    cmdline,
    ir_visitor::{Action, Construct, Visitor, VisitorData},
};
use fil_ir::{
    self as ir, Access, AddCtx, Bind, Command, Component, Connect, Context,
    Ctx, DenseIndexInfo, DisplayCtx, Expr, Foreign, Info, InvIdx, Invoke,
    Liveness, MutCtx, Port, PortIdx, PortOwner, Range, Subst, Time,
};
use itertools::Itertools;
use std::collections::HashMap;

// Eliminates bundle ports by breaking them into multiple len-1 ports, and eliminates local ports altogether.
pub struct BundleElim {
    context: DenseIndexInfo<Component, HashMap<PortIdx, Vec<PortIdx>>>,
    local_map: HashMap<(PortIdx, usize), (PortIdx, usize)>,
}

impl BundleElim {
    /// Gets corresponding ports from the context given a component and a port access.
    fn get(&self, access: &Access, data: &mut VisitorData) -> Vec<PortIdx> {
        let Access { port, start, end } = access;
        let start = start.concrete(&data.comp) as usize;
        let end = end.concrete(&data.comp) as usize;

        let mut ports = Vec::with_capacity(end - start);

        for idx in start..end {
            let mut group = (*port, idx);
            // loops until the non-local source of this port is found
            let (port, idx) = loop {
                match self.local_map.get(&group) {
                    Some(&g) => group = g,
                    None => break group,
                }
            };
            ports.push(self.context[data.idx][&port][idx]);
        }

        ports
    }

    /// Compiles a port by breaking it into multiple len-1 ports.
    fn port(&self, pidx: PortIdx, comp: &mut Component) -> Vec<PortIdx> {
        let one = comp.add(Expr::Concrete(1));
        let Port {
            owner,
            width,
            live,
            info,
        } = comp.get(pidx).clone();

        let Liveness { idx, len, range } = live;

        let start = comp.get(range.start).clone();
        let end = comp.get(range.end).clone();

        let len = len.concrete(comp);

        // if we need to preserve external interface information, we can't have bundle ports in the signature.
        if comp.src_info.is_some() && matches!(owner, PortOwner::Sig { .. }) {
            assert!(
                len == 1,
                "Bundle ports in the signature are not supported when external interface must be preserved."
            );

            // need to preserve the original portidx here to save the source information.
            return vec![pidx];
        }

        // creates the info to be cloned later.
        let info = comp.get(info).clone();

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

                let info = comp.add(info.clone());

                // adds the new port to the component and return its index
                comp.add(Port {
                    live,
                    owner,
                    info, // duplicate the info
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
                comp.get(idx).is_sig().then(|| {
                    log::debug!("{idx} is sig");
                    (idx, self.port(idx, comp))
                })
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
}

impl Construct for BundleElim {
    fn from(_opts: &cmdline::Opts, ctx: &mut Context) -> Self {
        let mut visitor = Self {
            context: DenseIndexInfo::default(),
            local_map: HashMap::new(),
        };
        // compiles signature ports and adds them to the context
        for (idx, c) in ctx.comps.iter_mut() {
            visitor.context.push(idx, visitor.sig(c));
        }

        visitor
    }

    fn clear_data(&mut self) {
        self.local_map.clear();
    }
}

impl Visitor for BundleElim {
    fn name() -> &'static str {
        "bundle-elim"
    }

    /// Compiles a connect command by breaking it into multiple simple connect commands
    /// Also eliminates local ports by storing their source bindings in the pass.
    fn connect(
        &mut self,
        connect: &mut Connect,
        data: &mut VisitorData,
    ) -> Action {
        let Connect { src, dst, .. } = connect;

        if !self.context.get(data.idx).contains_key(&dst.port) {
            // we are writing to a local port here.
            return Action::Change(vec![]);
        }

        // get the list of ports associated with each access in the connect.
        let src = self.get(src, data);
        let dst = self.get(dst, data);

        if src.len() != dst.len() {
            data.comp.internal_error(format!(
                "Mismatched access lengths for connect `{}`",
                data.comp.display(&*connect)
            ))
        }

        // split this single connects into `n` separate connects each with individual ports.
        // the local mapping optimization here works because it assumes that connects assigning to the local port
        // are defined before connects accessing the local port (I.E. assignments are in proper order).
        Action::Change(
            src.into_iter()
                .zip(dst)
                .map(|(src, dst)| {
                    Command::Connect(Connect {
                        src: Access::port(src, &mut data.comp),
                        dst: Access::port(dst, &mut data.comp),
                        info: data.comp.add(Info::empty()),
                    })
                })
                .collect(),
        )
    }

    fn bundle_def(
        &mut self,
        _: ir::PortIdx,
        _data: &mut VisitorData,
    ) -> Action {
        // Remove all bundle definitions
        Action::Change(vec![])
    }

    /// Compiles the body of a component and replaces all ports with their expanded versions.
    fn start(&mut self, data: &mut VisitorData) -> Action {
        let comp = &mut data.comp;
        // compile invocations
        for idx in comp.invocations().idx_iter() {
            let pl = self.inv(idx, comp);
            self.context.get_mut(data.idx).extend(pl);
        }

        // generates the local map for this component
        self.local_map = comp
            .cmds
            .iter()
            .filter_map(|cmd| {
                let Command::Connect(con) = cmd else {
                    return None;
                };
                let Connect { src, dst, .. } = con;

                // need to check validity here because already deleted ports are still in the connect commands
                if comp.ports().is_valid(dst.port)
                    && comp.get(dst.port).is_local()
                {
                    let dst_start = dst.start.concrete(comp) as usize;
                    let dst_end = dst.end.concrete(comp) as usize;
                    let src_start = src.start.concrete(comp) as usize;
                    let src_end = src.end.concrete(comp) as usize;
                    assert!(
                        dst_end - dst_start == src_end - src_start,
                        "Mismatched access lengths for connect `{}`",
                        comp.display(con)
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

        // delete local ports
        comp.ports()
            .iter()
            .filter(|(_, port)| port.is_local())
            .map(|(idx, _)| idx)
            .collect_vec() // collect here to remove ownership problems
            .into_iter()
            .for_each(|idx| comp.delete(idx)); // invalidate these ports

        Action::Continue
    }
}
