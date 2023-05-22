//! Convert the frontend AST to the IR.
use crate::{
    ast::{self, Id},
    ir,
    utils::Idx,
};
use std::collections::HashMap;

use super::{Ctx, Event, ExprIdx, Fact, Param, ParamIdx, Port, TimeIdx};

/// Structure to track name bindings through scopes
struct ScopeMap<V> {
    map: Vec<HashMap<Id, Idx<V>>>,
}
impl<V> ScopeMap<V> {
    fn new() -> Self {
        Self {
            map: vec![HashMap::new()],
        }
    }

    #[inline]
    /// Push a new scope level
    fn push(&mut self) {
        self.map.push(HashMap::new());
    }

    #[inline]
    /// Pop the last scope level
    fn pop(&mut self) {
        self.map.pop();
        assert!(!self.map.is_empty(), "Cannot pop last scope level");
    }

    /// Insert binding into the scope level
    fn insert(&mut self, id: Id, idx: Idx<V>) {
        self.map.last_mut().unwrap().insert(id, idx);
    }

    /// Return the value by searching through the scope levels
    fn get(&self, id: &Id) -> Option<&Idx<V>> {
        for scope in self.map.iter().rev() {
            if let Some(val) = scope.get(id) {
                return Some(val);
            }
        }
        None
    }
}

impl<V> std::ops::Index<&Id> for ScopeMap<V> {
    type Output = Idx<V>;

    fn index(&self, id: &Id) -> &Self::Output {
        self.get(id).unwrap()
    }
}

/// The signature of component.
///
/// A signature defines the parameters, ports, and facts all of which are added
/// to the component instantiating the signature.
struct Sig {
    params: Vec<Param>,
    ports: Vec<Port>,
    facts: Vec<Fact>,
}

/// Track the defined signature in the current scope.
struct SigMap {
    map: HashMap<Id, Sig>,
}
impl SigMap {
    fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }
    fn insert(&mut self, id: Id, sig: Sig) {
        self.map.insert(id, sig);
    }
    fn get(&self, id: &Id) -> Option<&Sig> {
        self.map.get(id)
    }
}
impl std::ops::Index<&Id> for SigMap {
    type Output = Sig;

    fn index(&self, id: &Id) -> &Self::Output {
        self.get(id).unwrap()
    }
}

/// Context used while building the IR.
struct BuildCtx<'a> {
    comp: &'a mut ir::Component,

    // Mapping from names to IR nodes.
    param_map: ScopeMap<Param>,
    event_map: ScopeMap<Event>,
    port_map: ScopeMap<Port>,
}

impl<'a> BuildCtx<'a> {
    fn new(comp: &'a mut ir::Component) -> Self {
        Self {
            comp,
            param_map: ScopeMap::new(),
            event_map: ScopeMap::new(),
            port_map: ScopeMap::new(),
        }
    }

    /// Push a new scope level
    #[inline]
    fn push(&mut self) {
        self.param_map.push();
        self.event_map.push();
        self.port_map.push();
    }

    /// Pop the last scope level
    #[inline]
    fn pop(&mut self) {
        self.param_map.pop();
        self.event_map.pop();
        self.port_map.pop();
    }

    /// Perform some action within a new scope
    fn with_scope<T, F>(&mut self, f: F) -> T
    where
        F: FnOnce(&mut Self) -> T,
    {
        self.push();
        let out = f(self);
        self.pop();
        out
    }

    fn expr(&mut self, expr: ast::Expr) -> ExprIdx {
        match expr {
            ast::Expr::Abstract(p) => {
                let Some(&pidx) = self.param_map.get(&p) else {
                    unreachable!("Parameter {p} not found")
                };
                self.comp.add(ir::Expr::Param(pidx))
            }
            ast::Expr::Concrete(n) => {
                let e = ir::Expr::Concrete(n);
                self.comp.add(e)
            }
            ast::Expr::Op { op, left, right } => {
                let l = self.expr(*left);
                let r = self.expr(*right);
                match op {
                    ast::Op::Add => l.add(r, self.comp),
                    ast::Op::Mul => l.mul(r, self.comp),
                    ast::Op::Sub => l.sub(r, self.comp),
                    ast::Op::Div => l.div(r, self.comp),
                    ast::Op::Mod => l.rem(r, self.comp),
                }
            }
            ast::Expr::App { func, arg } => {
                let arg = self.expr(*arg);
                match func {
                    ast::UnFn::Pow2 => arg.pow2(self.comp),
                    ast::UnFn::Log2 => arg.log2(self.comp),
                }
            }
        }
    }

    /// Add a parameter to the component.
    fn param(&mut self, param: ast::Id) -> ParamIdx {
        // TODO(rachit): Parameters do not have default values yet.
        let idx = self.comp.add(Param::default());
        self.param_map.insert(param, idx);
        idx
    }

    fn time(&mut self, t: ast::Time) -> TimeIdx {
        let Some(event) = self.event_map.get(&t.event).copied() else {
            unreachable!("Event {} not found", t.event)
        };
        let offset = self.expr(t.offset);
        self.comp.add(ir::Time { event, offset })
    }

    fn timesub(&mut self, ts: ast::TimeSub) -> ir::TimeSub {
        match ts {
            ast::TimeSub::Unit(e) => ir::TimeSub::Unit(self.expr(e)),
            ast::TimeSub::Sym { l, r } => {
                let l = self.time(l);
                let r = self.time(r);
                l.sub(r, self.comp)
            }
        }
    }

    /// Add an event to the component.
    fn event(&mut self, eb: ast::EventBind) {
        let delay = self.timesub(eb.delay.take());
        let default = eb.default.map(|t| self.time(t));
        let e = ir::Event { delay, default };
        let idx = self.comp.add(e);
        self.event_map.insert(*eb.event, idx);
    }

    fn range(&mut self, r: ast::Range) -> ir::Range {
        let start = self.time(r.start);
        let end = self.time(r.end);
        ir::Range { start, end }
    }

    fn port(&mut self, pd: ast::PortDef, owner: ir::PortOwner) {
        match pd {
            ast::PortDef::Port {
                name,
                liveness,
                bitwidth,
            } => {
                // The bundle type uses a fake bundle index and has a length of 1.
                // We don't need to push a new scope because this type is does not
                // bind any new parameters.
                let live = self.with_scope(|ctx| ir::Liveness {
                    idx: ParamIdx::UNKNOWN,
                    len: ctx.comp.num(1),
                    range: ctx.range(liveness.take()),
                });

                let p = ir::Port {
                    width: self.expr(bitwidth.take()),
                    owner,
                    live,
                };
                let idx = self.comp.add(p);
                self.port_map.insert(*name, idx);
            }
            ast::PortDef::Bundle(ast::Bundle {
                name,
                typ:
                    ast::BundleType {
                        idx,
                        len,
                        liveness,
                        bitwidth,
                    },
            }) => {
                // Construct the bundle type in a new scope.
                let live = self.with_scope(|ctx| ir::Liveness {
                    idx: ctx.param(*idx),
                    len: ctx.expr(len.take()),
                    range: ctx.range(liveness.take()),
                });

                let p = ir::Port {
                    width: self.expr(bitwidth.take()),
                    owner,
                    live,
                };
                let idx = self.comp.add(p);
                self.port_map.insert(name.take(), idx);
            }
        }
    }

    fn sig(&mut self, sig: ast::Signature) {
        for port in sig.inputs() {
            // XXX(rachit): Unnecessary clone.
            self.port(port.inner().clone(), ir::PortOwner::sig_in());
        }
        for port in sig.outputs() {
            // XXX(rachit): Unnecessary clone.
            self.port(port.inner().clone(), ir::PortOwner::sig_out());
        }

        for param in sig.params {
            self.param(param.copy());
        }
        for event in sig.events {
            self.event(event.take());
        }
    }

    fn instance(&mut self, inst: ast::Instance) {
        let ast::Instance {
            name,
            component,
            bindings,
        } = inst;
    }

    fn invoke(&mut self, inv: ast::Invoke) {
        let ast::Invoke {
            name,
            instance,
            abstract_vars,
            ports,
        } = inv;
        todo!()
    }

    fn commands(&mut self, cmds: Vec<ast::Command>) {
        for cmd in cmds {
            self.command(cmd);
        }
    }

    fn command(&mut self, cmd: ast::Command) {
        match cmd {
            ast::Command::Invoke(inv) => self.invoke(inv),
            ast::Command::Instance(inst) => self.instance(inst),
            ast::Command::Fact(_) => todo!(),
            ast::Command::Connect(_) => todo!(),
            ast::Command::ForLoop(_) => todo!(),
            ast::Command::If(_) => todo!(),
            ast::Command::Bundle(_) => todo!(),
        }
    }
}

fn comp(comp: ast::Component) -> ir::Component {
    let mut comp = ir::Component::default();
    todo!()
}
