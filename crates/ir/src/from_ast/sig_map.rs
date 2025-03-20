use super::BuildRes;
use crate as ir;
use fil_ast as ast;
use fil_utils::{self as utils, Error, Id};
use itertools::Itertools;
use std::collections::HashMap;

#[derive(Clone)]
/// The signature of component.
///
/// A signature defines the ports which are added to the component instantiating
/// the signature.
pub struct Sig {
    pub idx: ir::CompIdx,
    /// Mapping from source representation of input port to its IR index
    pub inputs: Vec<(ast::Loc<ast::PortDef>, ir::PortIdx)>,
    /// Mapping from source representation of output port to its IR index
    pub outputs: Vec<(ast::Loc<ast::PortDef>, ir::PortIdx)>,
    /// Mapping from sig bindings to parameter defined by it (if any)
    pub sig_binding: Vec<(ast::SigBind, Option<ir::ParamIdx>)>,
    // Existentially bound parameters defined by the component
    // pub exist_params: Vec<ir::ParamIdx>,
    /// The AST representation of parameters in the signature. Used to construct binding for the signature.
    pub raw_params: Vec<ast::ParamBind>,
    /// The AST representation of events in the signature
    pub raw_events: Vec<ast::EventBind>,
}

impl Sig {
    pub fn new(idx: ir::CompIdx, sig: &ast::Signature) -> Self {
        Self {
            idx,
            raw_params: sig.params.iter().map(|p| p.clone().take()).collect(),
            raw_events: sig.events.iter().map(|e| e.clone().take()).collect(),
            // Filled in later
            sig_binding: Vec::default(),
            inputs: Vec::default(),
            outputs: Vec::default(),
        }
    }

    /// Construct a param binding from this Signature's parameters and the given
    /// arguments.
    ///
    /// Fills in the missing arguments with default values and any parameters
    /// bound in the sig binding.
    pub fn param_binding(
        &self,
        args: impl IntoIterator<Item = ast::Expr>,
        comp: ast::Loc<Id>,
        diag: &mut utils::Diagnostics,
    ) -> BuildRes<ast::Binding<ast::Expr>> {
        let args = args.into_iter().collect_vec();
        let arg_len = args.len();

        // We've been given too many arguments
        if self.raw_params.len() < arg_len {
            let msg = format!(
                "`{}' requires at most {} parameters but {} were provided",
                comp.inner(),
                self.raw_params.len(),
                arg_len
            );
            let err = Error::malformed(msg.clone())
                .add_note(diag.add_info(msg, comp.pos()));
            diag.add_error(err);
            return Err(std::mem::take(diag));
        }

        let min_args = self
            .raw_params
            .iter()
            .take_while(|ev| ev.default.is_none())
            .count();

        // We don't have enough parameters. Generate error.
        if min_args > arg_len {
            let msg = format!(
                "`{}' requires at least {min_args} parameters but {arg_len} were provided",
                comp.inner(),
            );
            let err = Error::malformed(msg.clone())
                .add_note(diag.add_info(msg, comp.pos()));
            diag.add_error(err);
            return Err(std::mem::take(diag));
        }

        let mut partial_map = ast::Binding::new(
            self.raw_params
                .iter()
                .map(|pb| pb.name())
                .zip(args.iter().cloned()),
        );

        // Skip the events that have been bound and fill in the rest with
        // default values
        let remaining = self
            .raw_params
            .iter()
            .skip(args.len())
            .map(|pb| {
                let bind =
                    pb.default.as_ref().unwrap().clone().resolve(&partial_map);
                (pb.name(), bind)
            })
            .collect_vec();

        partial_map.extend(remaining);

        Ok(partial_map)
    }

    /// Construct an event binding from this Signature's events and the given
    /// arguments.
    /// Fills in the missing arguments with default values
    pub fn event_binding(
        &self,
        args: impl IntoIterator<Item = ast::Time>,
        inst: &ast::Loc<Id>,
        diag: &mut utils::Diagnostics,
    ) -> BuildRes<ast::Binding<ast::Time>> {
        let args = args.into_iter().collect_vec();

        // Too many arguments for the event
        if args.len() > self.raw_events.len() {
            let msg = format!(
                "`{}' requires at most {} events but {} were provided",
                inst.inner(),
                self.raw_events.len(),
                args.len()
            );
            let err = Error::malformed(msg.clone());
            let err = err.add_note(diag.add_info(msg, inst.pos()));
            diag.add_error(err);
            return Err(std::mem::take(diag));
        }

        let min_args = self
            .raw_events
            .iter()
            .take_while(|ev| ev.default.is_none())
            .count();

        // To few arguments for the event
        if min_args > args.len() {
            let msg = format!(
                "`{}' requires at least {} events but {} were provided",
                inst.inner(),
                min_args,
                args.len()
            );
            let err = Error::malformed(msg.clone());
            let err = err.add_note(diag.add_info(msg, inst.pos()));
            diag.add_error(err);
            return Err(std::mem::take(diag));
        }

        let mut partial_map = ast::Binding::new(
            self.raw_events
                .iter()
                .map(|eb| eb.event.inner())
                .cloned()
                .zip(args.iter().cloned()),
        );

        // Skip the events that have been bound
        let remaining = self
            .raw_events
            .iter()
            .skip(args.len())
            .map(|eb| {
                let bind = eb
                    .default
                    .as_ref()
                    .unwrap()
                    .clone()
                    .resolve_event(&partial_map);
                (*eb.event.inner(), bind)
            })
            .collect_vec();

        partial_map.extend(remaining);

        Ok(partial_map)
    }
}

#[derive(Default)]
/// Track the defined signatures in the current scope.
/// Mapping from names of component to [Sig].
pub struct SigMap {
    map: HashMap<Id, Sig>,
}

impl SigMap {
    /// Gets the signature if bound
    pub fn get(&self, id: &Id) -> Option<&Sig> {
        self.map.get(id)
    }
}

impl FromIterator<(Id, Sig)> for SigMap {
    fn from_iter<T: IntoIterator<Item = (Id, Sig)>>(iter: T) -> Self {
        let mut default = Self::default();

        for (id, sig) in iter.into_iter() {
            default.map.insert(id, sig);
        }

        default
    }
}

impl std::iter::Extend<(Id, Sig)> for SigMap {
    fn extend<I: IntoIterator<Item = (Id, Sig)>>(&mut self, iter: I) {
        let sm: SigMap = iter.into_iter().collect();
        self.map.extend(sm.map);
    }
}
