use super::Rewriter;
use crate::{
    core::{self, Loc},
    utils::{Binding, Traversal},
};
use itertools::Itertools;
use std::collections::{BTreeSet, HashMap, HashSet};

type Params = Vec<u64>;

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
        let idx = self.params[comp]
            .iter()
            .position(|p| p == param)
            .unwrap_or_else(|| {
                panic!("Parameter `{param}' not found in `{comp}'")
            });
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
            inst_params.params.insert(
                *comp.sig.name.inner(),
                comp.sig.params.iter().map(|p| p.copy()).collect(),
            );

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

#[derive(Default)]
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

    /// Transform the signature of a monomorphized component and give it a unique name using its parameters.
    fn sig(
        &mut self,
        sig: &core::Signature,
        binding: Vec<core::Expr>,
    ) -> core::Signature {
        // XXX: Short-circuit if binding is empty
        let name = Loc::unknown(Self::generate_mono_name(&sig.name, &binding));
        let mut nsig = sig.clone().resolve_exprs(binding);
        nsig.name = name;
        nsig.params.clear();
        nsig
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
                        new_binding.insert(idx.copy(), i.into());
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
        &mut self,
        comp: &core::Component,
        binding: &Binding<core::Expr>,
        externals: &HashSet<core::Id>,
    ) -> core::Component {
        let sig = self.sig(&comp.sig, binding.values().cloned().collect_vec());
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
        core::Component { sig, body }
    }

    /// Monomorphize the program by generate a component for each parameter of each instance.
    pub fn transform(ns: core::Namespace) -> core::Namespace {
        let mut mono = Self::default();
        let (mut inst_params, old_ns) = InstanceParams::build(ns);
        let mut ns = core::Namespace {
            components: Vec::new(),
            ..old_ns
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
                            comp.sig.params.iter().map(|p| p.copy()).zip(
                                bind_assigns.into_iter().map(|v| v.into()),
                            ),
                        );
                    let comp = mono.generate_comp(&comp, &binding, &externals);
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
                let comp = mono.generate_comp(
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
