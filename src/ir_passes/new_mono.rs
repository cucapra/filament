use crate::ir::{self, Ctx, MutCtx};
use itertools::Itertools;
use linked_hash_map::LinkedHashMap;
use std::collections::HashMap;

/// Represents a deferred monomorphize instance that will be processed by the pass at a later time.
struct MonoDeferred<'a, 'pass: 'a> {
    /// The name of the monomorphized component
    base: ir::Component,
    /// The underlying component to be monomorphized
    underlying: &'a ir::Component,
    /// Mapping from parameters in the underlying component to their constant bindings.
    binding: ir::Bind<ir::ParamIdx, u64>,

    /// Underlying pointer
    pass: &'a mut Monomorphize<'pass>,
}

impl<'a, 'pass: 'a> MonoDeferred<'a, 'pass> {
    fn param(&mut self, param: ir::ParamIdx) -> ir::ParamIdx {
        todo!()
    }

    /// Translates an ExprIdx defined by `underlying` to correponding one in `base`.
    fn expr(&mut self, expr: ir::ExprIdx) -> ir::ExprIdx {
        match self.underlying.get(expr).clone() {
            ir::Expr::Param(p) => self
                .binding
                .get(&p)
                .map(|n| self.base.num(*n))
                .unwrap_or_else(|| self.param(p).expr(&mut self.base)),
            ir::Expr::Concrete(n) => self.base.num(n),
            ir::Expr::Bin { op, lhs, rhs } => {
                let lhs = self.expr(lhs);
                let rhs = self.expr(rhs);
                self.base.add(ir::Expr::Bin { op, lhs, rhs })
            }
            ir::Expr::Fn { op, args } => todo!(),
        }
    }

    /// Monomorphize the `inst` (owned by self.underlying) and add it to `self.base`, and return the corresponding index
    fn instance(&mut self, inst: ir::InstIdx) -> ir::InstIdx {
        let ir::Instance { comp, params } = self.underlying.get(inst);
        let conc_params = params
            .iter()
            .map(|p| self.expr(*p).as_concrete(&self.base).unwrap())
            .collect_vec();
        let (comp, params) = self.pass.should_process(*comp, conc_params);
        let new_inst = ir::Instance {
            comp,
            params: params.into_iter().map(|n| self.base.num(n)).collect(),
        };
        self.base.add(new_inst)
    }

    fn command(&mut self, cmd: &ir::Command) -> ir::Command {
        match cmd {
            ir::Command::Instance(idx) => self.instance(*idx).into(),
            _ => todo!(),
        }
    }
}

/// Monomorphize the Filament program
pub struct Monomorphize<'a> {
    /// The new context
    ctx: ir::Context,

    /// The old context
    old: &'a ir::Context,
    // Names of external components
    externals: Vec<ir::CompIdx>,

    /// Instances that have already been processed. Tracks the name of the generated component
    processed: HashMap<(ir::CompIdx, Vec<u64>), ir::CompIdx>,
    /// Instances that need to be generated
    queue: LinkedHashMap<(ir::CompIdx, Vec<u64>), ir::CompIdx>,
}

impl Monomorphize<'_> {
    /// Queue an instance for processing by the pass.
    /// The processing happens at a later point but, if needed, the pass immediately allocates a new [ir::Component] and returns information to construct a new instance.
    fn should_process(
        &mut self,
        comp: ir::CompIdx,
        params: Vec<u64>,
    ) -> (ir::CompIdx, Vec<u64>) {
        // If this component doesn't need monomorphization, return the comp index.
        if self.externals.contains(&comp) || !Self::needs_monomorphize(comp) {
            return (comp, params);
        }
        let key = (comp, params);

        // If we've already processed this or queued this for processing, return the component
        if let Some(&name) =
            self.processed.get(&key).or_else(|| self.queue.get(&key))
        {
            return (name, vec![]);
        }

        // Otherwise, construct a new component and add it to the processing queue
        let new_comp = self.ctx.comp(false);
        self.queue.insert(key, new_comp);
        (new_comp, vec![])
    }

    fn next(&mut self) -> Option<MonoDeferred> {
        let Some(((underlying, params), base)) = self.queue.pop_front() else {
            return None;
        };
        let underlying = self.old.get(underlying);
        let binding = underlying
            .sig_params()
            .into_iter()
            .zip(params)
            .collect_vec();
        let base = std::mem::take(self.ctx.get_mut(base));
        MonoDeferred {
            base,
            underlying,
            binding: ir::Bind::new(binding),
            pass: self,
        };
        todo!()
    }

    /// Checks if a component needs to be monomorphized. This is the case if:
    /// - It has ANY parameters
    /// - If it uses loops, conditionals, or any other control constructs
    fn needs_monomorphize(comp: ir::CompIdx) -> bool {
        todo!()
    }
}

impl Monomorphize<'_> {
    /// Monomorphize the context by tracing starting from the top-level component.
    /// Returns an empty context if there is no top-level component.
    pub fn transform(ctx: ir::Context) -> ir::Context {
        todo!()
    }
}
