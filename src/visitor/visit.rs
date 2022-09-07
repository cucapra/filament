use crate::{
    errors::{Error, FilamentResult, WithPos},
    event_checker::ast,
};
use itertools::Itertools;
use std::collections::HashMap;

/// An Instance that has been resolved
pub enum ResolvedInstance<'a> {
    Bound {
        sig: &'a ast::Signature<ast::PortParam>,
        binds: Vec<u64>,
    },
    Concrete {
        sig: &'a ast::Signature<u64>,
    },
}
impl<'a> ResolvedInstance<'a> {
    pub fn abstract_vars(&self) -> &[ast::Id] {
        match self {
            ResolvedInstance::Bound { sig, .. } => &sig.abstract_vars,
            ResolvedInstance::Concrete { sig } => &sig.abstract_vars,
        }
    }

    pub fn input_names(&self) -> Vec<ast::Id> {
        match self {
            ResolvedInstance::Bound { sig, .. } => {
                sig.inputs.iter().map(|pd| pd.name.clone()).collect()
            }
            ResolvedInstance::Concrete { sig } => {
                sig.inputs.iter().map(|pd| pd.name.clone()).collect()
            }
        }
    }

    pub fn interface_name(&self) -> Vec<ast::Id> {
        match self {
            ResolvedInstance::Bound { sig, .. } => sig
                .interface_signals
                .iter()
                .map(|id| id.name.clone())
                .collect(),
            ResolvedInstance::Concrete { sig } => sig
                .interface_signals
                .iter()
                .map(|id| id.name.clone())
                .collect(),
        }
    }

    pub fn get_interface(&self, event: &ast::Id) -> Option<&ast::InterfaceDef> {
        match self {
            ResolvedInstance::Bound { sig, .. } => sig.get_interface(event),
            ResolvedInstance::Concrete { sig } => sig.get_interface(event),
        }
    }

    pub fn resolve(&self) -> FilamentResult<ast::Signature<u64>> {
        match self {
            ResolvedInstance::Bound { sig, binds } => sig.resolve(binds),
            ResolvedInstance::Concrete { sig } => Ok((*sig).clone()),
        }
    }

    pub fn binding(
        &self,
        abs: &'a [ast::TimeRep],
    ) -> FilamentResult<ast::Binding> {
        match self {
            ResolvedInstance::Bound { sig, .. } => sig.binding(abs),
            ResolvedInstance::Concrete { sig } => sig.binding(abs),
        }
    }
}

/// Environment to store the current set of bindings
pub struct Bindings<'a> {
    /// Signatures for external definitions
    ext_sigs: HashMap<ast::Id, &'a ast::Signature<ast::PortParam>>,
    /// Signatures for components
    comps: Vec<ast::Component>,
}
impl<'a> Bindings<'a> {
    pub fn new(
        ext_sigs: HashMap<ast::Id, &'a ast::Signature<ast::PortParam>>,
    ) -> Self {
        Self {
            ext_sigs,
            comps: Vec::new(),
        }
    }

    pub fn add_component(&mut self, comp: ast::Component) {
        self.comps.push(comp);
    }

    /// Get a binding associated with a name
    pub fn get(
        &'a self,
        name: &ast::Id,
        binds: &[u64],
    ) -> FilamentResult<ResolvedInstance> {
        if let Some(sig) = self.ext_sigs.get(name) {
            Ok(ResolvedInstance::Bound {
                sig,
                binds: binds.to_vec(),
            })
        } else {
            self.comps
                .iter()
                .find(|c| c.sig.name == name)
                .map(|comp| ResolvedInstance::Concrete { sig: &comp.sig })
                .ok_or_else(|| {
                    Error::malformed(format!("No binding for {}", name))
                })
        }
    }

    pub fn add_comp(&mut self, comp: ast::Component) {
        self.comps.push(comp);
    }
}
impl From<Bindings<'_>> for Vec<ast::Component> {
    fn from(bind: Bindings<'_>) -> Self {
        bind.comps
    }
}

/// Transform the given AST
pub trait Transform
where
    Self: Sized,
{
    /// Whether this component should be visited or not
    fn component_filter(&self, comp: &ast::Component) -> bool;

    /// Construct an instance of this pass
    fn new(_: &ast::Namespace) -> Self;

    #[inline]
    fn connect(
        &mut self,
        con: ast::Connect,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![con.into()])
    }

    #[inline]
    fn instance(
        &mut self,
        inst: ast::Instance,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![inst.into()])
    }

    #[inline]
    fn fsm(&mut self, fsm: ast::Fsm) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![fsm.into()])
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        inv: ast::Invoke,
        _: &ResolvedInstance,
    ) -> FilamentResult<Vec<ast::Command>> {
        Ok(vec![inv.into()])
    }

    #[inline]
    fn signature<W: Clone>(
        &mut self,
        sig: ast::Signature<W>,
    ) -> FilamentResult<ast::Signature<W>> {
        Ok(sig)
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        Ok(comp)
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        comp: ast::Component,
    ) -> FilamentResult<ast::Component> {
        Ok(comp)
    }

    /// Perform the component traversal
    fn component(
        &mut self,
        comp: ast::Component,
        binds: &Bindings,
    ) -> FilamentResult<ast::Component> {
        // Binding for instances
        let mut instances: HashMap<ast::Id, ResolvedInstance> = HashMap::new();
        let ast::Component { sig, body } = self.enter_component(comp)?;
        let body: Vec<ast::Command> = body
            .into_iter()
            .map(|cmd| match cmd {
                crate::core::Command::Invoke(inv) => {
                    let sig = instances.get(&inv.instance).unwrap();
                    self.invoke(inv, sig)
                }
                crate::core::Command::Instance(inst) => {
                    let sig = binds
                        .get(&inst.component, &inst.bindings)
                        .map_err(|err| {
                            err.add_note(
                                "Instance defined here",
                                inst.copy_span(),
                            )
                        })?;
                    instances.insert(inst.name.clone(), sig);
                    self.instance(inst)
                }
                crate::core::Command::Connect(con) => self.connect(con),
                crate::core::Command::Fsm(fsm) => self.fsm(fsm),
            })
            .collect::<FilamentResult<Vec<Vec<_>>>>()?
            .into_iter()
            .flatten()
            .collect_vec();

        let comp = ast::Component {
            body,
            sig: self.signature(sig)?,
        };
        self.exit_component(comp)
    }

    fn transform(mut ns: ast::Namespace) -> FilamentResult<ast::Namespace> {
        let comps = ns.components.drain(..).collect_vec();
        let mut binds = Bindings::new(ns.signatures());

        for comp in comps {
            let mut pass = Self::new(&ns);
            let ncomp = if pass.component_filter(&comp) {
                pass.component(comp, &binds)?
            } else {
                comp
            };
            binds.add_comp(ncomp);
        }

        Ok(ast::Namespace {
            components: binds.into(),
            imports: ns.imports,
            externs: ns.externs,
        })
    }
}
