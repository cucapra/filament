use super::Rewriter;
use crate::{core, errors::Error, passes::Pass, utils::Binding};
use itertools::Itertools;
use linked_hash_set::LinkedHashSet;
use std::collections::HashSet;

/// Monomorphize the Filament program
pub struct Monomorphize<'e> {
    /// Instances that have already been processed
    processed: HashSet<(core::Id, Vec<u64>)>,
    /// Instances that need to be generated
    queue: LinkedHashSet<(core::Id, Vec<u64>)>,
    /// Names of external components
    externals: &'e HashSet<core::Id>,
}

impl<'e> Monomorphize<'e> {
    fn new(externals: &'e HashSet<core::Id>) -> Self {
        Self {
            queue: LinkedHashSet::new(),
            processed: HashSet::new(),
            externals,
        }
    }

    /// Gnerate name for a monomorphized component based on the binding parameters.
    fn generate_mono_name(comp: &core::Id, params: &[u64]) -> core::Id {
        let suf = params.iter().map(|p| format!("_{}", p)).join("");
        format!("{comp}{suf}").into()
    }

    /// Coerce a list of expressions into a list of concrete values.
    fn coerce_params<'a>(
        params: impl IntoIterator<Item = &'a core::Expr>,
    ) -> Vec<u64> {
        params
            .into_iter()
            .map(|p| {
                u64::try_from(p).unwrap_or_else(|_| {
                    panic!("Parameter must be concrete but was {p}")
                })
            })
            .collect()
    }

    /// Return the name associated with the (component, params)
    // XXX: We should stop using generate_mono_name here
    fn get_name(&self, comp: core::Id, params: &[u64]) -> core::Id {
        Self::generate_mono_name(&comp, params)
    }

    /// Add instance for processing
    fn add_instance<'a>(
        &mut self,
        comp: core::Id,
        params: impl IntoIterator<Item = &'a core::Expr>,
    ) -> core::Id {
        let conc = Self::coerce_params(params);
        if self.processed.contains(&(comp, conc.clone())) {
            log::warn!("{}[{:?}] already processed", comp, conc);
            return self.get_name(comp, &conc);
        }
        let gen_name = Self::generate_mono_name(&comp, &conc);
        let key = (comp, conc);
        self.queue.insert(key);
        gen_name
    }

    /// Process the next instance in the queue. We mark it as processed
    /// and assume that it has been added to the namespace.
    fn process_instance(&mut self) -> Option<(core::Id, Vec<u64>)> {
        let inst = self.queue.pop_back()?;
        self.processed.insert(inst.clone());
        Some(inst)
    }

    /// Transform the signature of a monomorphized component and give it a unique name using its parameters.
    fn sig(
        &mut self,
        sig: &core::Signature,
        binding: Vec<core::Expr>,
    ) -> core::Signature {
        let name = self
            .get_name(sig.name.copy(), &Self::coerce_params(binding.iter()))
            .into();
        let mut nsig = sig.clone().resolve_exprs(binding);
        nsig.name = name;
        // Remove the parameters from the signature
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
        &mut self,
        commands: impl Iterator<Item = core::Command>,
        // Binding for the parameters of the component.
        // Must only contain concrete values
        param_binding: &Binding<core::Expr>,
        // Current set of bound names
        mut prev_names: Binding<core::Id>,
        // Current suffix
        suffix: &str,
    ) -> Vec<core::Command> {
        let mut n_cmds = Vec::new();
        for cmd in commands {
            match cmd {
                core::Command::Assume(core::Assume { cons }) => {
                    match cons.clone().take().eval(param_binding) {
                        Ok(true) => (),
                        Ok(false) => {
                            panic!("Assumption violated during elaboration.")
                        }
                        Err(e) => {
                            panic!(
                                "Assumption `{}' violated: {}. Bindings: {:?}",
                                cons.inner(),
                                e.kind,
                                param_binding
                            )
                        }
                    }
                }
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

                    if self.externals.contains(&component) {
                        n_cmds.push(
                            core::Instance::new(name, component, resolved)
                                .into(),
                        );
                    } else {
                        // If this is a component, replace the instance name with the monomorphized version
                        let new_name = self
                            .add_instance(
                                component.copy(),
                                resolved
                                    .iter()
                                    .map(|p| p.inner())
                                    .collect_vec(),
                            )
                            .into();
                        n_cmds.push(
                            core::Instance::new(name, new_name, vec![]).into(),
                        );
                    }
                }
                core::Command::If(core::If { cond, then, alt }) => {
                    let cond = cond.eval(param_binding).unwrap();
                    let cmds = if cond { then } else { alt };
                    n_cmds.extend(self.commands(
                        cmds.into_iter(),
                        param_binding,
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
                        .unwrap_or_else(|e: Error| {
                            panic!(
                                "loop start must be concrete but was {}",
                                e.kind
                            )
                        });
                    let e = end
                        .resolve(param_binding)
                        .try_into()
                        .unwrap_or_else(|e: Error| {
                            panic!(
                                "loop end must be concrete but was {}",
                                e.kind
                            )
                        });

                    for i in s..e {
                        let mut new_binding = (*param_binding).clone();
                        new_binding.insert(idx.copy(), i.into());
                        // Recur on the body of the loop
                        let ncmds = self.commands(
                            body.iter().cloned(),
                            &new_binding,
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
                    unreachable!("FSMs cannot be monomorphized")
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
        let body =
            self.commands(comp.body.iter().cloned(), binding, prev_names, "");
        core::Component { sig, body }
    }
}

impl Pass for Monomorphize<'_> {
    /// Monomorphize the program by generate a component for each parameter of each instance.
    fn transform(mut ns: core::Namespace) -> core::Namespace {
        let Some(top_idx) = ns.main_idx() else {
            log::warn!("program has no main component so resulting program will be empty");
            ns.components.clear();
            return ns;
        };

        // Start the process by monomorphizing the main component
        let main = ns.components.remove(top_idx);
        let externals =
            ns.signatures().map(|(_, sig)| *sig.name.inner()).collect();
        let mut mono = Monomorphize::new(&externals);
        let mut comps = vec![mono.generate_comp(&main, &Binding::new(None))];

        while let Some((name, params)) = mono.process_instance() {
            log::trace!(
                "processing {}[{}]",
                name,
                params.iter().map(|p| p.to_string()).join(", "),
            );
            // Get the component associated with the instance
            let comp = ns
                .components
                .iter()
                .find(|c| *c.sig.name.inner() == name)
                .unwrap();
            // Generate binding for the component
            let binding = Binding::new(
                comp.sig
                    .params
                    .iter()
                    .map(|p| p.copy())
                    .zip(params.into_iter().map(|v| v.into())),
            );
            comps.push(mono.generate_comp(comp, &binding));
        }
        drop(mono);

        comps.reverse();
        ns.components = comps;
        ns
    }
}
