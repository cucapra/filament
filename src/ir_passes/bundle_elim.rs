use crate::{
    cmdline,
    ir_visitor::{Action, Construct, Visitor, VisitorData},
};
use fil_ir::{
    self as ir, Access, AddCtx, Bind, Command, Component, Connect, Ctx,
    DenseIndexInfo, DisplayCtx, Expr, Foreign, Info, InvIdx, Invoke, Liveness,
    MutCtx, Port, PortIdx, PortOwner, Range, Subst, Time,
};
use fil_utils as utils;
use itertools::Itertools;
use std::collections::HashMap;

pub type PortInfo =
    (/*lens=*/ Vec<usize>, /*gen_ports=*/ Vec<PortIdx>);

// Eliminates bundle ports by breaking them into multiple len-1 ports, and eliminates local ports altogether.
pub struct BundleElim {
    /// Mapping from component to the map from signature bundle port to generated port.
    context: DenseIndexInfo<Component, HashMap<PortIdx, PortInfo>>,
    /// Mapping from index into a dst port to an index of the src port.
    local_map: HashMap<
        (PortIdx, /*idxs=*/ Vec<usize>),
        (PortIdx, /*idxs=*/ Vec<usize>),
    >,
}

impl BundleElim {
    /// Gets corresponding ports from the context given a component and a port access.
    fn get(&self, access: &Access, data: &mut VisitorData) -> Vec<PortIdx> {
        let Access { port, ranges } = access;
        let comp = &data.comp;
        let mut len = 1;
        let ranges_c = ranges
            .iter()
            .map(|(s, e)| {
                let s = s.concrete(comp) as usize;
                let e = e.concrete(comp) as usize;
                len *= e - s;
                (s, e)
            })
            .collect_vec();

        let mut ports = Vec::with_capacity(len);

        let comp_info = &self.context[data.idx];

        for idx in utils::all_indices(ranges_c) {
            let mut group = &(*port, idx);
            // loops until the non-local source of this port is found
            let (port, idxs) = loop {
                match self.local_map.get(group) {
                    Some(g) => group = g,
                    None => break group,
                }
            };
            let (lens, sig_ports) = &comp_info[port];
            ports.push(sig_ports[utils::flat_idx(idxs, lens)])
        }

        ports
    }

    /// Compiles a port by breaking it into multiple len-1 ports.
    fn port(&self, pidx: PortIdx, comp: &mut Component) -> PortInfo {
        let one = comp.add(Expr::Concrete(1));
        let Port {
            owner,
            width,
            live,
            info,
        } = comp.get(pidx).clone();

        let Liveness { idxs, lens, range } = live;

        let start = comp.get(range.start).clone();
        let end = comp.get(range.end).clone();

        // The total size of the bundle
        let lens = lens.iter().map(|l| l.concrete(comp) as usize).collect_vec();
        let len = lens.iter().product::<usize>();

        // if we need to preserve external interface information, we can't have bundle ports in the signature.
        if comp.src_info.is_some() && matches!(owner, PortOwner::Sig { .. }) {
            assert!(
                len == 1,
                "Bundle ports in the signature are not supported when external interface must be preserved."
            );

            // need to preserve the original portidx here to save the source information.
            return (lens, vec![pidx]);
        }

        // creates the info to be cloned later.
        let info = comp.get(info).clone();

        // create a single port for each element in the bundle.
        let ports = (0..len)
            .map(|i| {
                let binding = Bind::new(
                    utils::nd_idx(i, &lens).into_iter().zip_eq(&idxs).map(
                        |(v, idx)| (*idx, comp.add(Expr::Concrete(v as u64))),
                    ),
                );

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
                    idxs: vec![idxs[0]], // this should technically be some null parameter, as it will refer to a deleted parameter now.
                    lens: vec![one],
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
                                self.context[owner][&key].1[i],
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
        for idx in idxs {
            comp.delete(idx);
        }
        (lens, ports)
    }

    /// Compiles the signature of a component and adds the new ports to the context mapping.
    fn sig(&self, comp: &mut Component) -> HashMap<PortIdx, PortInfo> {
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
    ) -> HashMap<PortIdx, PortInfo> {
        let Invoke { ports, .. } = comp.get_mut(idx);
        // first take all the old ports and split them up
        let mappings = std::mem::take(ports)
            .into_iter()
            .map(|p| (p, self.port(p, comp)))
            .collect_vec();

        // add them back to the invoke (need to get mutably again because comp is mutated above)
        comp.get_mut(idx).ports =
            mappings.iter().flat_map(|(_, (_, v))| v).copied().collect();

        mappings.into_iter().collect()
    }
}

impl Construct for BundleElim {
    fn from(_opts: &cmdline::Opts, ctx: &mut ir::Context) -> Self {
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
                .zip_eq(dst)
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

        // Generates the local map for this component
        // At this point, we expect all the connects to be at the top-level because monomorphization has eliminated all control flow.
        self.local_map = comp
            .cmds
            .iter()
            .filter_map(|cmd| {
                let ir::Command::Connect(ir::Connect { src, dst, .. }) = cmd
                else {
                    return None;
                };

                // need to check validity here because already deleted ports are still in the connect commands
                if !(comp.ports().is_valid(dst.port)
                    && comp.get(dst.port).is_local())
                {
                    return None;
                }

                let src_ranges = src
                    .ranges
                    .iter()
                    .map(|(s, e)| {
                        let s = s.concrete(comp) as usize;
                        let e = e.concrete(comp) as usize;
                        (s, e)
                    })
                    .collect_vec();
                let dst_ranges = dst
                    .ranges
                    .iter()
                    .map(|(s, e)| {
                        let s = s.concrete(comp) as usize;
                        let e = e.concrete(comp) as usize;
                        (s, e)
                    })
                    .collect_vec();

                Some(
                    utils::all_indices(dst_ranges)
                        .into_iter()
                        .zip_eq(utils::all_indices(src_ranges))
                        .map(|(d, s)| ((dst.port, d), (src.port, s))),
                )
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
