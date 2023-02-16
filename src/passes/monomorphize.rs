use crate::{
    core,
    utils::{Binding, PostOrder},
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};

type Params = Vec<u64>;

#[derive(Default)]
/// Parameters used for each instance of a Filament-level components
struct InstanceParams {
    /// Parameters for a component
    params: HashMap<core::Id, Vec<core::Id>>,
    /// The parameters for the component
    bindings: HashMap<core::Id, Vec<Params>>,
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
            .map(|p|
                match p.abs.len() {
                    0 => vec![p.concrete],
                    1 => self.param_values(parent, &p.abs[0]).collect(),
                    n => unreachable!("Cannot have more than one abstract parameter in a binding: {n}")
                }
            )
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
            match p.concrete() {
                Some(p) => name += format!("_{}", p).as_str(),
                None => {
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

    fn commands(
        commands: impl Iterator<Item = core::Command>,
        // Binding for the parameters of the component.
        // Must only contain concrete values
        param_binding: &Binding<core::Expr>,
        externals: &HashSet<core::Id>,
    ) -> Vec<core::Command> {
        commands
            .map(|cmd| match cmd {
                core::Command::Invoke(core::Invoke {
                    name,
                    instance,
                    abstract_vars,
                    ports,
                    ..
                }) => core::Invoke::new(
                    name,
                    instance,
                    abstract_vars
                        .into_iter()
                        .map(|t| t.resolve_expr(param_binding))
                        .collect_vec(),
                    ports,
                )
                .into(),
                core::Command::Connect(con) => con.into(),
                core::Command::Fsm(fsm) => fsm.into(),
                core::Command::Instance(inst) => {
                    let core::Instance {
                        name,
                        component,
                        bindings,
                        ..
                    } = inst;
                    let resolved = bindings
                        .into_iter()
                        .map(|p| p.resolve(param_binding))
                        .collect();

                    if externals.contains(&component) {
                        core::Instance::new(name, component, resolved).into()
                    } else {
                        // If this is a component, replace the instance name with the monomorphized version
                        core::Instance::new(
                            name,
                            Self::generate_mono_name(&component, &resolved),
                            vec![],
                        )
                        .into()
                    }
                }
            })
            .collect_vec()
    }

    /// Generate a new component using the binding parameters.
    fn generate_comp(
        comp: &core::Component,
        binding: &Binding<core::Expr>,
        externals: &HashSet<core::Id>,
    ) -> core::Component {
        let sig =
            Self::sig(&comp.sig, &binding.values().cloned().collect_vec());
        let body =
            Self::commands(comp.body.iter().cloned(), binding, externals);
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
                // If we have a component with parameters but not bindings, it was probably unused.
                if !comp.sig.params.is_empty() {
                    log::warn!("skipping monomorphization for unused parameteric component `{}'", comp.sig.name);
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
