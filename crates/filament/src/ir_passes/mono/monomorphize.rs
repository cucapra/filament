use super::{
    Base, CompKey, InstanceInfo, IntoBase, IntoUdl, MonoDeferred, MonoSig,
    Underlying, UnderlyingComp,
};
use fil_gen as gen;
use fil_ir::{self as ir, Ctx, IndexStore};
use fil_utils::{self as utils, AttrCtx};
use ir::{AddCtx, EntryPoint};
use itertools::Itertools;
use std::collections::HashMap;

/// The Monomorphize pass.
///
/// ## Basic Strategy
///
/// The pass recusively monomorphizes all user-level Filament components and
/// replaces their uses with monomorphic variants. For example, in the program:
/// ```
/// comp Foo[A](...) { .. }
/// comp Bar(...) {
///     F0 := new Foo[10]
///     F1 := new Foo[20]
/// }
/// ```
/// will be turned into:
/// ```
/// comp Foo_10(...) { .. }
/// comp Foo_20(...) { .. }
/// comp Bar() {
///     F0 := new Foo_10
///     F1 := new Foo_20
/// }
/// ```
///
/// Primitive uses are not mormorphized because their parameterization happens
/// outside of Filament.
///
/// ## Existential Parameters
///
/// Existential parameters get their bindings from evaluating a component's body.
/// One way to think about parameters is that normal parameters in the
/// component's signature are "inputs" while existential parameters are
/// "outputs".
///
/// Because of this, the pass must monomorphize all instances as soon as it sees them:
/// ```
/// comp Foo {
///    B0 := new Bar[10];
///    B1 := new Baz[B0::Out + 15]; // B0 must be fully monomorphized before we can monomorphize B1
/// }
/// ```
///
/// ## Struct Information
///
/// Information generated while monomorphizing a program. This tracks the global
/// set of information generated during monomorphization while [super::MonoSig]
/// tracks information generated for a single component.
pub struct Monomorphize<'a> {
    /// The new context
    pub ctx: ir::Context,
    /// The old context
    pub old: &'a ir::Context,
    // Names of external components
    pub externals: Vec<ir::CompIdx>,
    /// Instances that have already been processed. Tracks the name of the generated component
    pub processed: HashMap<CompKey, Base<ir::Component>>,
    /// Mapping from old ports to new ports, for resolving Foreigns
    inst_info: HashMap<CompKey, InstanceInfo>,
    /// Tracks which components are defined in which files
    pub ext_map: HashMap<String, Vec<ir::CompIdx>>,
    /// Generator executor
    gen_exec: &'a mut Option<gen::GenExec>,
}

impl<'a> Monomorphize<'a> {
    fn new(
        old: &'a ir::Context,
        gen_exec: &'a mut Option<gen::GenExec>,
    ) -> Self {
        Monomorphize {
            ctx: ir::Context::default(),
            old,
            externals: vec![],
            processed: HashMap::new(),
            inst_info: HashMap::new(),
            ext_map: HashMap::new(),
            gen_exec,
        }
    }
}

impl<'ctx> Monomorphize<'ctx> {
    /// Returns a reference to the instance info for a component.
    /// **Panics** if the instance does not exist.
    pub fn inst_info(&self, comp_key: &CompKey) -> &InstanceInfo {
        self.inst_info.get(comp_key).unwrap_or_else(|| {
            unreachable!("instance not monormorphized: {comp_key}")
        })
    }

    /// Returns a mutable reference to the instance info for a component or a
    /// default value if it does not exist.
    pub fn inst_info_mut(&mut self, comp_key: CompKey) -> &mut InstanceInfo {
        self.inst_info.entry(comp_key).or_default()
    }

    /// Generate an component using the `gen` framework
    pub fn gen(
        &mut self,
        comp: Underlying<ir::Component>,
        params: Vec<u64>,
        key: CompKey,
    ) -> Base<ir::Component> {
        let underlying = self.old.get(comp.idx());
        let Some(is) = &underlying.src_info else {
            unreachable!("external component has no src_info")
        };
        let Some(tool) = &is.gen_tool else {
            unreachable!("gen component does not have a tool")
        };
        let inst = gen::Instance {
            name: is.name.as_ref().to_string(),
            parameters: params.iter().map(|p| p.to_string()).collect(),
        };

        // Running the tool returns the location of the Verilog file that
        // contains the definition and mapping for existential parameters.
        let gen::ToolOutput {
            name,
            file,
            exist_params,
        } = self
            .gen_exec
            .as_mut()
            .unwrap_or_else(|| unreachable!("no generate executor defined"))
            .gen_instance(tool, &inst);

        // Partially convert the signature
        let monosig =
            MonoSig::new(underlying, ir::CompType::External, comp, params);
        let mut mono_comp = MonoDeferred::new(
            UnderlyingComp::new(self.old.get(comp.idx())),
            self,
            monosig,
        );
        mono_comp.sig_partial_mono();

        // Add generated existential parameters
        let exists = exist_params
            .into_iter()
            .map(|(name, val)| {
                let v: u64 = val.parse().unwrap();
                let Some(param) = is.param_from_src_name(name.clone()) else {
                    unreachable!("component does not have parameter `{name}'")
                };
                // Add to the binding
                mono_comp.push_binding(param.ul(), v);
                (param.ul(), v)
            })
            .collect_vec();

        // Monomorphize the siganture of the component using the parameters
        mono_comp.sig_complete_mono();
        let mut comp = mono_comp.take();

        // Update the source name
        comp.src_info.as_mut().unwrap().name = name.into();

        // Add information about the existential parameters to the global map
        let info = self.inst_info_mut(key.clone());
        for (p, v) in exists {
            info.add_exist_val(p, v);
        }

        // Add component to the context
        let idx = self.ctx.add(comp).base();
        self.processed.insert(key, idx);

        // Add the component to the filemap
        self.ext_map
            .entry(file.to_string_lossy().to_string())
            .or_default()
            .push(idx.get());

        idx
    }

    /// Monomorphize an external component.
    /// External components can either be definitions to a specific Verilog file
    /// or generated from a tool.
    pub fn ext(
        &mut self,
        comp: Underlying<ir::Component>,
        key: CompKey,
    ) -> Base<ir::Component> {
        let underlying = self.old.get(comp.idx());
        let Some(filename) = self.old.get_filename(comp.idx()) else {
            unreachable!("external component has no filename")
        };

        // Clone the component
        let n_comp = underlying.clone();

        // Add information for the component
        let info = self.inst_info_mut(key.clone());
        for (port, _) in n_comp.ports().iter() {
            let old_port = port.ul();
            let new_port = port.base();
            info.add_port(old_port, new_port);
        }
        for (ev, _) in n_comp.events().iter() {
            let old_ev = ev.ul();
            let new_ev = ev.base();
            info.add_event(old_ev, new_ev);
        }

        // Add component information to processed map
        let idx = self.ctx.add(n_comp).base();
        self.processed.insert(key, idx);

        // Add the component to the filemap
        self.ext_map.entry(filename).or_default().push(idx.get());

        idx
    }

    /// Monomorphize a component and return its index in the new context.
    pub fn monomorphize(&mut self, ck: CompKey) -> Base<ir::Component> {
        log::debug!("Monomorphizing `{}'", ck.comp.idx());
        let CompKey { comp, params } = ck;
        let underlying = self.old.get(comp.idx());

        let n_ck: CompKey = if underlying.is_ext() && !underlying.is_gen() {
            (comp, vec![]).into()
        } else {
            (comp, params.clone()).into()
        };

        // If we've already processed this, return the component
        if let Some(&name) = self.processed.get(&n_ck) {
            return name;
        }

        if underlying.is_gen() {
            return self.gen(comp, params, n_ck);
        }

        // Copy the component signature if it is an external and return it.
        if underlying.is_ext() {
            return self.ext(comp, n_ck);
        }

        // Otherwise monomorphize the definition of the component
        let monosig =
            MonoSig::new(underlying, ir::CompType::Source, comp, params);

        // the component whose signature we want to monomorphize
        // Monomorphize the sig
        let mut mono_comp = MonoDeferred::new(
            UnderlyingComp::new(self.old.get(comp.idx())),
            self,
            monosig,
        )
        .comp();

        mono_comp.attrs.set(
            utils::CompBool::Monomorphic,
            true,
            utils::GPosIdx::UNKNOWN,
        );

        let new_comp = self.ctx.add(mono_comp).base();
        self.processed.insert(n_ck, new_comp);

        // return the `base` index so we can update the instance
        new_comp
    }
}

impl Monomorphize<'_> {
    /// Monomorphize the context by tracing starting from the top-level component.
    /// Returns an empty context if there is no top-level component.
    pub fn transform(
        ctx: &ir::Context,
        gen: &mut Option<gen::GenExec>,
    ) -> ir::Context {
        let Some(entrypoint) = &ctx.entrypoint else {
            log::warn!("Program has no entrypoint. Result will be empty.");
            return ir::Context {
                comps: IndexStore::default(),
                entrypoint: None,
                externals: HashMap::new(),
            };
        };
        let EntryPoint {
            comp: entrypoint,
            bindings,
        } = entrypoint;

        let entrypoint = entrypoint.ul();
        // Monomorphize the entrypoint
        let mut mono = Monomorphize::new(ctx, gen);
        let ck = CompKey::new(entrypoint, bindings.clone());
        mono.monomorphize(ck.clone());

        let new_entrypoint = mono.processed.get(&ck).unwrap();
        // New component no longer has any bindings
        mono.ctx.entrypoint = Some(EntryPoint::new(new_entrypoint.get()));
        mono.ctx.externals = mono.ext_map;
        ir::Validate::context(&mono.ctx);
        mono.ctx
    }
}
