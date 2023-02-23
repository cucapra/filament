use crate::{
    core,
    utils::{Binding, PostOrder},
};
use itertools::Itertools;
use std::collections::{BTreeSet, HashMap, HashSet};

type Params = Vec<u64>;

/// Rewrite commands based on the binding.
struct Rewriter {
    /// Mapping for bound names. Any name that is bound will not be renamed.
    binding: Binding<core::Id>,
    /// Suffix used to generate new names
    suffix: String,
}

impl Rewriter {
    fn new(binding: Binding<core::Id>, suffix: String) -> Self {
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
        match port.typ {
            core::PortType::InvPort { invoke, name } => {
                core::PortType::InvPort {
                    invoke: self.binding[&invoke].clone(),
                    name,
                }
            }
            t => t,
        }
        .into()
    }

    /// Generate new set of commands by renaming name based on the binding.
    fn rewrite(&mut self, cmds: Vec<core::Command>) -> Vec<core::Command> {
        // First rename all binders
        for cmd in &cmds {
            match cmd {
                core::Command::Invoke(inv) => self.add_name(inv.name.clone()),
                core::Command::Instance(inst) => {
                    self.add_name(inst.name.clone())
                }
                core::Command::Bundle(bl) => self.add_name(bl.name.clone()),
                core::Command::Fsm(_) => {
                    unreachable!("Cannot monomorphize FSMs")
                }
                core::Command::ForLoop(_) => {
                    unreachable!("Inner loops should be monomorphized already")
                }
                core::Command::Connect(_) => {}
            }
        }
        let mut n_cmds = Vec::with_capacity(cmds.len());
        // Rename all uses of the binders
        for cmd in cmds {
            match cmd {
                core::Command::Invoke(core::Invoke {
                    abstract_vars,
                    instance,
                    name,
                    ports,
                    ..
                }) => {
                    let name = self.binding[&name].clone();
                    let instance = self.binding[&instance].clone();
                    let ports: Option<Vec<core::Port>> = ports.map(|ps| {
                        ps.into_iter()
                            .map(|p| self.rewrite_port(p))
                            .collect_vec()
                    });
                    n_cmds.push(
                        core::Invoke::new(name, instance, abstract_vars, ports)
                            .into(),
                    )
                }
                core::Command::Instance(core::Instance {
                    name,
                    bindings,
                    component,
                    ..
                }) => {
                    n_cmds.push(
                        core::Instance::new(
                            self.binding[&name].clone(),
                            component,
                            bindings,
                        )
                        .into(),
                    );
                }
                core::Command::Connect(core::Connect {
                    src,
                    dst,
                    guard,
                    ..
                }) => {
                    assert!(guard.is_none(), "Cannot monomorphize guards");
                    n_cmds.push(
                        core::Connect::new(
                            self.rewrite_port(dst),
                            self.rewrite_port(src),
                            None,
                        )
                        .into(),
                    )
                }
                core::Command::Bundle(core::Bundle { name, len, typ }) => {
                    n_cmds.push(
                        core::Bundle::new(
                            self.binding[&name].clone(),
                            len,
                            typ,
                        )
                        .into(),
                    )
                }
                core::Command::ForLoop(_) => unreachable!(),
                core::Command::Fsm(_) => unreachable!(),
            }
        }
        n_cmds
    }
}

#[derive(Default)]
/// Parameters used for each instance of a Filament-level components
struct InstanceParams {
    /// Parameters for a component
    params: HashMap<core::Id, Vec<core::Id>>,
    /// The parameters for the component
    bindings: HashMap<core::Id, BTreeSet<Params>>,
}

impl InstanceParams {
    /// Returns all possible values a particular parameter can take in a component.
    fn param_values(
        &self,
        comp: &core::Id,
        param: &core::Id,
    ) -> impl Iterator<Item = u64> + '_ {
        // Find the index of the parameter in the component
        let idx = self.params[comp].iter().position(|p| p == param).unwrap();
        // Return all values that occur at that position
        self.bindings[comp].iter().map(move |binding| binding[idx])
    }

    /// Resolve and add all the bindings implied by an abstract binding.
    /// A binding will look like this: [W, 1, 2, K]
    /// In order to resolve it, we look at all possible values for `W` and `K` in the parent parameters
    fn add_params(
        &mut self,
        parent: &core::Id,
        comp: &core::Id,
        params: &[core::Expr],
    ) {
        // log::trace!("{} -> {} -> {}", parent, comp, params);
        // All possible values for each parameter computed by resolving each parameter that occurs in the binding
        let all_binds = params
            .iter()
            .map(|p| {
                let abs = p.exprs();
                match abs.len() {
                    0 => vec![u64::try_from(p).unwrap()],
                    1 => self.param_values(parent, &abs[0]).collect(),
                    n => unreachable!("Cannot have more than one abstract parameter in a binding: {n}")
                }
            })
            .multi_cartesian_product()
            .collect_vec();

        log::trace!("all_binds = {:?}", all_binds);

        self.bindings
            .entry(comp.clone())
            .or_default()
            .extend(all_binds);
    }
}

impl InstanceParams {
    fn build(ns: core::Namespace) -> (Self, core::Namespace) {
        let externals = ns
            .signatures()
            .map(|(_, sig)| sig.name.clone())
            .collect::<HashSet<_>>();

        let mut inst_params = InstanceParams::default();
        let mut order = PostOrder::from(ns);

        order.apply(|comp| {
            // Add parameters for this component
            inst_params
                .params
                .insert(comp.sig.name.clone(), comp.sig.params.clone());

            // Add bindings from each instance
            for command in &comp.body {
                if let core::Command::Instance(inst) = command {
                    if !externals.contains(&inst.component)
                        && !inst.bindings.is_empty()
                    {
                        inst_params.add_params(
                            &comp.sig.name,
                            &inst.component,
                            &inst.bindings,
                        );
                    }
                }
            }
        });

        (inst_params, order.take())
    }
}

/// Monomorphize the Filament program
pub struct Monomorphize;

impl Monomorphize {
    /// Gnerate name for a monomorphized component based on the binding parameters.
    fn generate_mono_name<'a>(
        comp: &core::Id,
        params: impl IntoIterator<Item = &'a core::Expr>,
    ) -> core::Id {
        let mut name = String::from(comp.as_ref());
        for p in params {
            match u64::try_from(p) {
                Ok(p) => name += format!("_{}", p).as_str(),
                Err(_) => {
                    unreachable!("Binding should only contain concrete values")
                }
            }
        }
        name.into()
    }

    fn sig(sig: &core::Signature, binding: &[core::Expr]) -> core::Signature {
        // XXX: Short-circuit if binding is empty
        let mut nsig = sig.resolve_offset(binding);
        nsig.name = Self::generate_mono_name(&sig.name, binding);
        nsig.params = vec![];
        nsig
    }

    fn connect(
        con: core::Connect,
        binding: &Binding<core::Expr>,
    ) -> core::Connect {
        core::Connect::new(
            con.dst.resolve_exprs(binding),
            con.src.resolve_exprs(binding),
            con.guard,
        )
    }

    fn commands(
        commands: impl Iterator<Item = core::Command>,
        // Binding for the parameters of the component.
        // Must only contain concrete values
        param_binding: &Binding<core::Expr>,
        // Name of components that should not be monomorphized.
        externals: &HashSet<core::Id>,
        // Current set of bound names
        mut prev_names: Binding<core::Id>,
        // Current suffix
        suffix: &str,
    ) -> Vec<core::Command> {
        let mut n_cmds = Vec::new();
        for cmd in commands {
            match cmd {
                core::Command::Bundle(bl) => {
                    n_cmds.push(bl.resolve_exprs(param_binding).into());
                }
                core::Command::Invoke(core::Invoke {
                    name,
                    instance,
                    abstract_vars,
                    ports,
                    ..
                }) => {
                    // Add identity mapping for name
                    prev_names.insert(name.clone(), name.clone());
                    // Resolve the expressions in the invoke
                    n_cmds.push(
                        core::Invoke::new(
                            name,
                            instance,
                            abstract_vars
                                .into_iter()
                                .map(|t| t.resolve_expr(param_binding))
                                .collect_vec(),
                            ports.map(|ps| {
                                ps.into_iter()
                                    .map(|p| p.resolve_exprs(param_binding))
                                    .collect_vec()
                            }),
                        )
                        .into(),
                    );
                }
                core::Command::Connect(con) => {
                    n_cmds.push(Self::connect(con, param_binding).into());
                }
                core::Command::Instance(inst) => {
                    let core::Instance {
                        name,
                        component,
                        bindings,
                        ..
                    } = inst;
                    // Add identity mapping for name
                    prev_names.insert(name.clone(), name.clone());

                    let resolved = bindings
                        .into_iter()
                        .map(|p| p.resolve(param_binding))
                        .collect();

                    if externals.contains(&component) {
                        n_cmds.push(
                            core::Instance::new(name, component, resolved)
                                .into(),
                        );
                    } else {
                        // If this is a component, replace the instance name with the monomorphized version
                        n_cmds.push(
                            core::Instance::new(
                                name,
                                Self::generate_mono_name(&component, &resolved),
                                vec![],
                            )
                            .into(),
                        );
                    }
                }
                core::Command::ForLoop(core::ForLoop {
                    idx,
                    start,
                    end,
                    body,
                    ..
                }) => {
                    // Compute the start and end values of the loop
                    let s: u64 =
                        start.resolve(param_binding).try_into().unwrap_or_else(
                            |_| panic!("Loop start must be concrete"),
                        );
                    let e =
                        end.resolve(param_binding).try_into().unwrap_or_else(
                            |_| panic!("Loop end must be concrete"),
                        );

                    for i in s..e {
                        let mut new_binding = (*param_binding).clone();
                        new_binding.insert(idx.clone(), i.into());
                        // Recur on the body of the loop
                        let ncmds = Self::commands(
                            body.iter().cloned(),
                            &new_binding,
                            externals,
                            prev_names.clone(),
                            suffix,
                        );
                        // Rewrite all names in the body and add them to the new commands
                        let mut rw = Rewriter::new(
                            prev_names.clone(),
                            format!("{suffix}{i}"),
                        );
                        n_cmds.extend(rw.rewrite(ncmds));
                    }
                }
                core::Command::Fsm(_) => {
                    unreachable!("Cannot mono FSMs ")
                }
            }
        }
        n_cmds
    }

    /// Generate a new component using the binding parameters.
    fn generate_comp(
        comp: &core::Component,
        binding: &Binding<core::Expr>,
        externals: &HashSet<core::Id>,
    ) -> core::Component {
        let sig =
            Self::sig(&comp.sig, &binding.values().cloned().collect_vec());
        let body = Self::commands(
            comp.body.iter().cloned(),
            binding,
            externals,
            Binding::default(),
            "",
        );
        core::Component { sig, body }
    }

    /// Monomorphize the program by generate a component for each parameter of each instance.
    pub fn transform(ns: core::Namespace) -> core::Namespace {
        let (mut inst_params, old_ns) = InstanceParams::build(ns);
        let mut ns = core::Namespace {
            imports: old_ns.imports,
            externs: old_ns.externs,
            components: Vec::new(),
        };

        let externals: HashSet<_> =
            ns.signatures().map(|(_, sig)| sig.name.clone()).collect();

        // For each parameter of each instance, generate a new component
        for comp in old_ns.components {
            if let Some(all_binds) = inst_params.bindings.remove(&comp.sig.name)
            {
                for bind_assigns in all_binds {
                    let binding =
                        Binding::new(
                            comp.sig.params.iter().cloned().zip(
                                bind_assigns.into_iter().map(|v| v.into()),
                            ),
                        );
                    let comp = Self::generate_comp(&comp, &binding, &externals);
                    ns.components.push(comp);
                }
            } else {
                // If we have a component with parameters but not bindings, it was not used.
                if !comp.sig.params.is_empty() {
                    continue;
                }
                let comp = Self::generate_comp(
                    &comp,
                    &Binding::new(std::iter::empty()),
                    &externals,
                );
                ns.components.push(comp);
            }
        }
        ns
    }
}
