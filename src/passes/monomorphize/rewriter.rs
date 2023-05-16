use crate::ast::{self, Loc};
use crate::utils::Binding;
use itertools::Itertools;

/// Rewrite commands based on the binding.
pub struct Rewriter {
    /// Mapping for bound names.
    /// Any name that is bound will not be renamed using the suffix.
    binding: Binding<ast::Id>,
    /// Suffix used to generate new names
    suffix: String,
}

impl Rewriter {
    pub fn new(binding: Binding<ast::Id>, suffix: String) -> Self {
        Self { binding, suffix }
    }

    /// Rename a name based on the binding.
    fn add_name(&mut self, name: ast::Id) {
        if self.binding.find(&name).is_none() {
            let n = format!("{}{}", name, self.suffix).into();
            self.binding.insert(name, n)
        }
    }

    fn rewrite_port(&mut self, port: ast::Port) -> ast::Port {
        match port {
            ast::Port::InvPort { invoke, name } => ast::Port::InvPort {
                invoke: Loc::unknown(self.binding[invoke.inner()]),
                name,
            },
            ast::Port::Bundle { name, access } => ast::Port::Bundle {
                name: Loc::unknown(self.binding[name.inner()]),
                access,
            },
            ast::Port::InvBundle {
                invoke,
                port,
                access,
            } => ast::Port::InvBundle {
                invoke: Loc::unknown(self.binding[invoke.inner()]),
                port,
                access,
            },
            p @ (ast::Port::This(_) | ast::Port::Constant(_)) => p,
        }
    }

    /// Generate new set of commands by renaming name based on the binding.
    /// The commands should have been fully evaluated and be free of
    /// assumptions, loops, and conditional statements.
    pub fn rewrite(&mut self, cmds: Vec<ast::Command>) -> Vec<ast::Command> {
        // First rename all binders
        for cmd in &cmds {
            match cmd {
                ast::Command::Invoke(inv) => self.add_name(*inv.name.inner()),
                ast::Command::Instance(inst) => {
                    self.add_name(*inst.name.inner())
                }
                ast::Command::Bundle(bl) => self.add_name(*bl.name.inner()),
                ast::Command::ForLoop(_) => {
                    unreachable!("Inner loops should be monomorphized already")
                }
                ast::Command::If(_) => {
                    unreachable!(
                        "If statements should be monomorphized already"
                    )
                }
                ast::Command::Connect(_) | ast::Command::Fact(_) => {}
            }
        }
        let mut n_cmds = Vec::with_capacity(cmds.len());
        // Rename all uses of the binders
        for cmd in cmds {
            let out = match cmd.clone() {
                ast::Command::Invoke(ast::Invoke {
                    abstract_vars,
                    instance,
                    name,
                    ports,
                    ..
                }) => {
                    let name = self.binding[&name];
                    let instance = self.binding[&instance];
                    let ports: Option<Vec<ast::Loc<ast::Port>>> =
                        ports.map(|ps| {
                            ps.into_iter()
                                .map(|p| p.map(|p| self.rewrite_port(p)))
                                .collect_vec()
                        });

                    ast::Invoke::new(
                        Loc::unknown(name),
                        Loc::unknown(instance),
                        abstract_vars,
                        ports,
                    )
                    .into()
                }
                ast::Command::Instance(ast::Instance {
                    name,
                    bindings,
                    component,
                    ..
                }) => ast::Instance::new(
                    Loc::unknown(self.binding[name.inner()]),
                    component,
                    bindings,
                )
                .into(),
                ast::Command::Connect(ast::Connect {
                    src, dst, guard, ..
                }) => {
                    assert!(guard.is_none(), "Cannot monomorphize guards");
                    ast::Connect::new(
                        dst.map(|p| self.rewrite_port(p)),
                        src.map(|p| self.rewrite_port(p)),
                        None,
                    )
                    .into()
                }
                ast::Command::Bundle(ast::Bundle { name, typ, .. }) => {
                    ast::Bundle::new(self.binding[name.inner()].into(), typ)
                        .into()
                }
                ast::Command::If(_)
                | ast::Command::ForLoop(_)
                | ast::Command::Fact(_) => unreachable!(),
            };
            n_cmds.push(out);
        }
        n_cmds
    }
}
