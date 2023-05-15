use crate::core::{self, Loc};
use crate::utils::Binding;
use itertools::Itertools;

/// Rewrite commands based on the binding.
pub struct Rewriter {
    /// Mapping for bound names.
    /// Any name that is bound will not be renamed using the suffix.
    binding: Binding<core::Id>,
    /// Suffix used to generate new names
    suffix: String,
}

impl Rewriter {
    pub fn new(binding: Binding<core::Id>, suffix: String) -> Self {
        Self { binding, suffix }
    }

    /// Rename a name based on the binding.
    fn add_name(&mut self, name: core::Id) {
        if self.binding.find(&name).is_none() {
            let n = format!("{}{}", name, self.suffix).into();
            self.binding.insert(name, n)
        }
    }

    fn rewrite_port(&mut self, port: core::Port) -> core::Port {
        match port {
            core::Port::InvPort { invoke, name } => core::Port::InvPort {
                invoke: Loc::unknown(self.binding[invoke.inner()]),
                name,
            },
            core::Port::Bundle { name, access } => core::Port::Bundle {
                name: Loc::unknown(self.binding[name.inner()]),
                access,
            },
            core::Port::InvBundle {
                invoke,
                port,
                access,
            } => core::Port::InvBundle {
                invoke: Loc::unknown(self.binding[invoke.inner()]),
                port,
                access,
            },
            p @ (core::Port::This(_) | core::Port::Constant(_)) => p,
        }
    }

    /// Generate new set of commands by renaming name based on the binding.
    /// The commands should have been fully evaluated and be free of
    /// assumptions, loops, and conditional statements.
    pub fn rewrite(&mut self, cmds: Vec<core::Command>) -> Vec<core::Command> {
        // First rename all binders
        for cmd in &cmds {
            match cmd {
                core::Command::Invoke(inv) => self.add_name(*inv.name.inner()),
                core::Command::Instance(inst) => {
                    self.add_name(*inst.name.inner())
                }
                core::Command::Bundle(bl) => self.add_name(*bl.name.inner()),
                core::Command::Fsm(_) => {
                    unreachable!("Cannot monomorphize FSMs")
                }
                core::Command::ForLoop(_) => {
                    unreachable!("Inner loops should be monomorphized already")
                }
                core::Command::If(_) => {
                    unreachable!(
                        "If statements should be monomorphized already"
                    )
                }
                core::Command::Connect(_) | core::Command::Fact(_) => {}
            }
        }
        let mut n_cmds = Vec::with_capacity(cmds.len());
        // Rename all uses of the binders
        for cmd in cmds {
            let out = match cmd.clone() {
                core::Command::Invoke(core::Invoke {
                    abstract_vars,
                    instance,
                    name,
                    ports,
                    ..
                }) => {
                    let name = self.binding[&name];
                    let instance = self.binding[&instance];
                    let ports: Option<Vec<core::Loc<core::Port>>> =
                        ports.map(|ps| {
                            ps.into_iter()
                                .map(|p| p.map(|p| self.rewrite_port(p)))
                                .collect_vec()
                        });

                    core::Invoke::new(
                        Loc::unknown(name),
                        Loc::unknown(instance),
                        abstract_vars,
                        ports,
                    )
                    .into()
                }
                core::Command::Instance(core::Instance {
                    name,
                    bindings,
                    component,
                    ..
                }) => core::Instance::new(
                    Loc::unknown(self.binding[name.inner()]),
                    component,
                    bindings,
                )
                .into(),
                core::Command::Connect(core::Connect {
                    src,
                    dst,
                    guard,
                    ..
                }) => {
                    assert!(guard.is_none(), "Cannot monomorphize guards");
                    core::Connect::new(
                        dst.map(|p| self.rewrite_port(p)),
                        src.map(|p| self.rewrite_port(p)),
                        None,
                    )
                    .into()
                }
                core::Command::Bundle(core::Bundle { name, typ, .. }) => {
                    core::Bundle::new(self.binding[name.inner()].into(), typ)
                        .into()
                }
                core::Command::If(_)
                | core::Command::ForLoop(_)
                | core::Command::Fact(_)
                | core::Command::Fsm(_) => unreachable!(),
            };
            n_cmds.push(out);
        }
        n_cmds
    }
}
