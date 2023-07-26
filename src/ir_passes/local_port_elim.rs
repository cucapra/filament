use std::collections::HashMap;

use crate::{
    ir::{Access, Command, Component, Connect, Ctx, Info, PortIdx},
    ir_visitor::{Action, Visitor},
};

/// Eliminates local ports from the IR.
#[derive(Default)]
pub struct LocalPortElim {
    port_map: HashMap<PortIdx, PortIdx>,
}

impl Visitor for LocalPortElim {
    /// loops over connect commands and adds local port connects to the mapping.
    fn start(&mut self, comp: &mut Component) -> Action {
        for (src, dst) in comp.cmds.iter().filter_map(|cmd| match cmd {
            Command::Connect(Connect { src, dst, .. }) => {
                Some((src.port, dst.port))
            }
            _ => None,
        }) {
            if comp.get(dst).is_local() {
                self.port_map.insert(dst, src);
            }
        }

        Action::Continue
    }

    fn connect(
        &mut self,
        connect: &mut Connect,
        comp: &mut Component,
    ) -> Action {
        let Connect {
            src: Access { port: src, .. },
            dst: Access { port: dst, .. },
            ..
        } = &connect;

        // Throw away this connect if it assigns to a local port.
        if comp.get(*dst).is_local() {
            return Action::Change(vec![]);
        }

        let mut src = *src;
        let src = loop {
            // end the loop once we reach a non-local port
            if !comp.get(src).is_local() {
                break src;
            }
            // loop back in the chain
            src = *self.port_map.get(&src).unwrap();
        };

        Action::Change(vec![Command::Connect(Connect {
            src: Access::port(src, comp),
            dst: Access::port(*dst, comp),
            info: comp.add(Info::Empty),
        })])
    }
}
