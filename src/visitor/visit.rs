use super::{Bindings, ResolvedInstance};
use crate::{errors::FilamentResult, event_checker::ast};
use itertools::Itertools;
use std::collections::HashMap;

/// Transform the given AST
pub trait Transform
where
    Self: Sized,
{
    /// Extra information needed to construct this visitor.
    type Info;

    /// Construct an instance of this pass
    fn new(_: &ast::Namespace, info: &Self::Info) -> Self;

    /// What data should be cleared between component
    fn clear_data(&mut self);

    /// Whether this component should be visited or not
    fn component_filter(&self, comp: &ast::Component) -> bool;

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
                    let sig =
                        binds.get_component(&inst.component, &inst.bindings);
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

    fn transform(
        mut ns: ast::Namespace,
        info: Self::Info,
    ) -> FilamentResult<(ast::Namespace, Self)> {
        let comps = ns.components.drain(..).collect_vec();
        let mut binds = Bindings::new(ns.signatures());
        let mut pass = Self::new(&ns, &info);

        for comp in comps {
            pass.clear_data();
            let ncomp = if pass.component_filter(&comp) {
                pass.component(comp, &binds)?
            } else {
                comp
            };
            binds.add_component(ncomp);
        }

        Ok((
            ast::Namespace {
                components: binds.into(),
                imports: ns.imports,
                externs: ns.externs,
            },
            pass,
        ))
    }
}
