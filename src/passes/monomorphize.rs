use crate::{
    core::{self, Loc},
    utils::{Binding, Traversal},
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
        match port {
            core::Port::InvPort { invoke, name } => core::Port::InvPort {
                invoke: Loc::unknown(self.binding[invoke.inner()]),
                name,
            },
            core::Port::Bundle { name, idx } => core::Port::Bundle {
                name: Loc::unknown(self.binding[name.inner()]),
                idx,
            },
            t => t,
        }
    }

    /// Generate new set of commands by renaming name based on the binding.
    fn rewrite(&mut self, cmds: Vec<core::Command>) -> Vec<core::Command> {
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
                core::Command::Connect(_) => {}
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
                core::Command::Bundle(core::Bundle {
                    name, len, typ, ..
                }) => core::Bundle::new(
                    Loc::unknown(self.binding[name.inner()]),
                    len,
                    typ,
                )
                .into(),
                core::Command::If(_) => todo!(),
                core::Command::ForLoop(_) => unreachable!(),
                core::Command::Fsm(_) => unreachable!(),
            };
            n_cmds.push(out);
        }
        n_cmds
    }
}

#[derive(Default)]
/// Parameters used for each instance of a Filament-level components
struct InstanceParams {
    /// Parameters for a component
    params: HashMap<core::Id, Vec<core::Id>>,
    /// The parameters discovered for each component
    /// Maps component name to all parameter bindings used for it
    bindings: HashMap<core::Id, BTreeSet<Params>>,
}

impl InstanceParams {
    /// Returns all possible values a particular parameter can take in a component.
    fn param_values(
        &self,
        comp: &core::Id,
        param: &core::Id,
    ) -> Box<dyn Iterator<Item = u64> + '_> {
        // Find the index of the parameter in the component
        let idx = self.params[comp].iter().position(|p| p == param).unwrap();
        // Return all values that occur at that position
        if let Some(params) = self.bindings.get(comp) {
            Box::new(params.iter().map(move |binding| binding[idx]))
        } else {
            log::trace!("No binding for {comp}");
            Box::new(std::iter::empty())
        }
    }

    /// Resolve and add all the bindings implied by an abstract binding.
    /// A binding will look like this: [W, 1, 2, K]
    /// In order to resolve it, we look at all possible values for `W` and `K` in the parent parameters
    fn add_params(
        &mut self,
        parent: &core::Id,
        comp: &core::Id,
        params: &[Loc<core::Expr>],
    ) {
        // All possible values for each parameter computed by resolving each parameter that occurs in the binding
        let all_binds = params
            .iter()
            .map(|p| {
                let abs = p.exprs().collect_vec();
                match abs.len() {
                    0 => vec![u64::try_from(p.inner()).unwrap()],
                    1 => self.param_values(parent, abs[0]).collect(),
                    n => unreachable!("Cannot have more than one abstract parameter in a binding: {n}")
                }
            })
            .multi_cartesian_product()
            .unique()
            .collect_vec();

        log::trace!("{parent} -> {comp} -> {all_binds:?}");

        self.bindings.entry(*comp).or_default().extend(all_binds);
    }
}

impl InstanceParams {
    fn process_cmd(
        comp: &core::Id,
        cmd: &core::Command,
        inst_params: &mut InstanceParams,
        externals: &HashSet<core::Id>,
    ) {
        match cmd {
            core::Command::Instance(inst) => {
                if !externals.contains(&inst.component)
                    && !inst.bindings.is_empty()
                {
                    inst_params.add_params(
                        comp,
                        &inst.component,
                        &inst.bindings,
                    );
                }
            }
            core::Command::ForLoop(fl) => {
                for cmd in &fl.body {
                    Self::process_cmd(comp, cmd, inst_params, externals);
                }
            }
            core::Command::If(i) => {
                for cmd in &i.then {
                    Self::process_cmd(comp, cmd, inst_params, externals);
                }
                for cmd in &i.alt {
                    Self::process_cmd(comp, cmd, inst_params, externals);
                }
            }
            core::Command::Bundle(_)
            | core::Command::Connect(_)
            | core::Command::Fsm(_)
            | core::Command::Invoke(_) => (),
        }
    }

    fn build(ns: core::Namespace) -> (Self, core::Namespace) {
        let externals = ns
            .signatures()
            .map(|(_, sig)| *sig.name.inner())
            .collect::<HashSet<_>>();

        let mut inst_params = InstanceParams::default();
        let mut order = Traversal::from(ns);

        order.apply_post_order(|comp| {
            // Add parameters for this component
            inst_params
                .params
                .insert(*comp.sig.name.inner(), comp.sig.params.clone());

            // Add bindings from each instance
            for cmd in &comp.body {
                Self::process_cmd(
                    &comp.sig.name,
                    cmd,
                    &mut inst_params,
                    &externals,
                )
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
                Err(n) => {
                    unreachable!("Binding contains non-concrete value: {n}")
                }
            }
        }
        name.into()
    }

    /// Transform the signature of a monomorphized component and generate any assignments needed to
    /// implement the bundles mentioned in the signature.
    /// - Any IO bundles are moved into the component body.
    /// - Input bundles generate assignments from the bundle to the port
    /// - Output bundles generate assignments from the port to the bundles
    fn sig(
        sig: &core::Signature,
        binding: Vec<core::Expr>,
    ) -> (core::Signature, Vec<core::Command>, Vec<core::Command>) {
        // To add before and after the body
        let (mut pre_cmds, mut post_cmds) = (vec![], vec![]);
        // XXX: Short-circuit if binding is empty
        let name = Loc::unknown(Self::generate_mono_name(&sig.name, &binding));
        let mut nsig = sig.clone().resolve_exprs(binding);
        nsig.name = name.clone();
        nsig.params = vec![];
        // Generate ports for each bundle
        let sig = nsig.replace_ports(&mut |p, is_input| {
            let pos = p.pos();
            match p.take() {
                p @ core::PortDef::Port { .. } => vec![Loc::new(p, pos)],
                core::PortDef::Bundle(core::Bundle {
                    name: bundle_name,
                    len,
                    typ,
                }) => {
                    // Add bundle to the top-level commands
                    pre_cmds.push(
                        core::Bundle::new(
                            bundle_name.clone(),
                            len.clone(),
                            typ.clone(),
                        )
                        .into(),
                    );
                    // Extract the bundle type
                    let core::BundleType {
                        idx,
                        liveness,
                        bitwidth,
                    } = typ;
                    let len: u64 = len.take().try_into().unwrap_or_else(|s| {
                        panic!("Failed to concretize `{s}'")
                    });
                    // For each index in the bundle, generate a corresponding port
                    (0..len)
                        .map(|i| {
                            let bind = Binding::new(vec![(
                                idx,
                                core::Expr::concrete(i),
                            )]);
                            let liveness =
                                liveness.clone().take().resolve_exprs(&bind);
                            let name = Loc::unknown(core::Id::from(format!(
                                "{}_{i}",
                                bundle_name.clone()
                            )));
                            let port = Loc::unknown(core::PortDef::Port {
                                name: name.clone(),
                                liveness: Loc::unknown(liveness),
                                bitwidth: bitwidth.clone(),
                            });
                            let this_port =
                                Loc::unknown(core::Port::This(name));
                            let bundle_port = Loc::unknown(core::Port::bundle(
                                bundle_name.clone(),
                                Loc::unknown(core::Expr::concrete(i)),
                            ));
                            // Generate assignment for the bundle
                            if is_input {
                                // bundle{i} = this.p
                                pre_cmds.push(core::Command::Connect(
                                    core::Connect::new(
                                        bundle_port,
                                        this_port,
                                        None,
                                    ),
                                ))
                            } else {
                                // this.p = bundle{i}
                                post_cmds.push(core::Command::Connect(
                                    core::Connect::new(
                                        this_port,
                                        bundle_port,
                                        None,
                                    ),
                                ))
                            };
                            port
                        })
                        .collect_vec()
                }
            }
        });
        (sig, pre_cmds, post_cmds)
    }

    fn connect(
        con: core::Connect,
        binding: &Binding<core::Expr>,
    ) -> core::Connect {
        core::Connect::new(
            con.dst.map(|e| e.resolve_exprs(binding)),
            con.src.map(|e| e.resolve_exprs(binding)),
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
                    prev_names.insert(*bl.name.inner(), *bl.name.inner());
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
                    prev_names.insert(*name.inner(), *name.inner());
                    // Resolve the expressions in the invoke
                    n_cmds.push(
                        core::Invoke::new(
                            name,
                            instance,
                            abstract_vars
                                .into_iter()
                                .map(|t| {
                                    t.map(|t| t.resolve_expr(param_binding))
                                })
                                .collect_vec(),
                            ports.map(|ps| {
                                ps.into_iter()
                                    .map(|p| {
                                        p.map(|p| {
                                            p.resolve_exprs(param_binding)
                                        })
                                    })
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
                    prev_names.insert(*name.inner(), *name.inner());

                    let resolved = bindings
                        .into_iter()
                        .map(|p| p.map(|p| p.resolve(param_binding)))
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
                                Loc::unknown(Self::generate_mono_name(
                                    &component,
                                    resolved.iter().map(|p| p.inner()),
                                )),
                                vec![],
                            )
                            .into(),
                        );
                    }
                }
                core::Command::If(core::If { cond, then, alt }) => {
                    let cond = cond.eval(param_binding);
                    let cmds = if cond { then } else { alt };
                    n_cmds.extend(Self::commands(
                        cmds.into_iter(),
                        param_binding,
                        externals,
                        prev_names.clone(),
                        suffix,
                    ));
                }
                core::Command::ForLoop(core::ForLoop {
                    idx,
                    start,
                    end,
                    body,
                    ..
                }) => {
                    // Compute the start and end values of the loop
                    let s: u64 = start
                        .resolve(param_binding)
                        .try_into()
                        .unwrap_or_else(|e| {
                            panic!("loop start must be concrete but was {e}")
                        });
                    let e =
                        end.resolve(param_binding).try_into().unwrap_or_else(
                            |e| panic!("loop end must be concrete but was {e}"),
                        );

                    for i in s..e {
                        let mut new_binding = (*param_binding).clone();
                        new_binding.insert(idx, i.into());
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
        let (sig, mut pre_cmds, post_cmds) =
            Self::sig(&comp.sig, binding.values().cloned().collect_vec());
        // Map all port names to themselves
        let prev_names = Binding::new(
            comp.sig
                .ports()
                .iter()
                .map(|p| {
                    let n = *p.inner().name().inner();
                    (n, n)
                })
                .collect_vec(),
        );
        let body = Self::commands(
            comp.body.iter().cloned(),
            binding,
            externals,
            prev_names,
            "",
        );
        pre_cmds.extend(body);
        pre_cmds.extend(post_cmds);
        core::Component {
            sig,
            body: pre_cmds,
        }
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
            ns.signatures().map(|(_, sig)| *sig.name.inner()).collect();

        // For each parameter of each instance, generate a new component
        for comp in old_ns.components {
            if let Some(all_binds) = inst_params.bindings.remove(&comp.sig.name)
            {
                assert!(
                    !all_binds.is_empty(),
                    "No bindings for component {}",
                    comp.sig.name
                );
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
                    log::trace!(
                        "Parameterized component `{}' is not used",
                        &comp.sig.name
                    );
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
