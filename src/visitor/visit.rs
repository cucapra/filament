use super::Bindings;
use crate::{
    core::{self, TimeRep, WidthRep},
    errors::FilamentResult,
    visitor,
};
use itertools::Itertools;
use std::collections::HashMap;

/// Transform the given AST
pub trait Transform<T, W>
where
    Self: Sized,
    T: TimeRep,
    W: WidthRep,
{
    /// Extra information needed to construct this visitor.
    type Info;

    /// Construct an instance of this pass
    fn new(_: &core::Namespace<T, W>, info: &Self::Info) -> Self;

    /// What data should be cleared between component
    fn clear_data(&mut self);

    /// Whether this component should be visited or not
    fn component_filter(&self, comp: &core::Component<T, W>) -> bool;

    #[inline]
    fn connect(
        &mut self,
        con: core::Connect,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![con.into()])
    }

    #[inline]
    fn instance(
        &mut self,
        inst: core::Instance<W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![inst.into()])
    }

    #[inline]
    fn fsm(
        &mut self,
        fsm: core::Fsm,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![fsm.into()])
    }

    /// Transform an invoke statement. Provides access to the signature of the
    /// component that is being invoked.
    #[inline]
    fn invoke(
        &mut self,
        inv: core::Invoke<T>,
        _: &visitor::ResolvedInstance<T, W>,
    ) -> FilamentResult<Vec<core::Command<T, W>>> {
        Ok(vec![inv.into()])
    }

    #[inline]
    fn signature(
        &mut self,
        sig: core::Signature<T, W>,
    ) -> FilamentResult<core::Signature<T, W>> {
        Ok(sig)
    }

    /// Perform computation before the component traversal
    #[inline]
    fn enter_component(
        &mut self,
        comp: core::Component<T, W>,
    ) -> FilamentResult<core::Component<T, W>> {
        Ok(comp)
    }

    /// Perform computation after the component traversal
    #[inline]
    fn exit_component(
        &mut self,
        comp: core::Component<T, W>,
    ) -> FilamentResult<core::Component<T, W>> {
        Ok(comp)
    }

    /// Perform the component traversal
    fn component(
        &mut self,
        comp: core::Component<T, W>,
        binds: &visitor::Bindings<T, W>,
    ) -> FilamentResult<core::Component<T, W>> {
        // Binding for instances
        let mut instances: HashMap<core::Id, visitor::ResolvedInstance<T, W>> =
            HashMap::new();
        let core::Component { sig, body } = self.enter_component(comp)?;
        let body: Vec<core::Command<_, _>> = body
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

        let comp = core::Component {
            body,
            sig: self.signature(sig)?,
        };
        self.exit_component(comp)
    }

    fn transform(
        mut ns: core::Namespace<T, W>,
        info: Self::Info,
    ) -> FilamentResult<(core::Namespace<T, W>, Self)> {
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
            core::Namespace {
                components: binds.into(),
                imports: ns.imports,
                externs: ns.externs,
            },
            pass,
        ))
    }
}
