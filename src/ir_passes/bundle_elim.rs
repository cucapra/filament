use std::collections::HashMap;

use crate::{
    cmdline,
    ir::{
        Component, Context, Ctx, DenseIndexInfo, Expr, Liveness, MutCtx, Port,
        PortIdx, Range, Time,
    },
};

#[derive(Default)]
pub struct BundleElim {
    context: DenseIndexInfo<Component, HashMap<PortIdx, Vec<PortIdx>>>,
}

impl BundleElim {
    fn sig_port(pidx: PortIdx, comp: &mut Component) -> Vec<PortIdx> {
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

                let owner = owner.clone();

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

    // ignored because of false positive
    #[allow(clippy::needless_collect)]
    /// Compiles the signature of a component and adds the new ports to the context mapping.
    fn sig(comp: &mut Component) -> HashMap<PortIdx, Vec<PortIdx>> {
        // need to collect here because of ownership issues.
        let ports: Vec<_> = comp
            .ports()
            .idx_iter()
            .filter(|p| comp.get(*p).is_sig())
            .collect();

        ports
            .into_iter()
            .map(|idx| (idx, Self::sig_port(idx, comp)))
            .collect()
    }

    fn do_pass(opts: &cmdline::Opts, ctx: &mut Context) {
        let mut visitor = Self::default();
        for (idx, c) in ctx.comps.iter_mut() {
            visitor.context.push(idx, Self::sig(c));
        }
    }
}
