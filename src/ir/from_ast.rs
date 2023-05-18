//! Convert the frontend AST to the IR.
use crate::{
    ast::{self, Id},
    ir,
};
use std::collections::HashMap;

use super::{
    expr, Ctx, Event, EventIdx, ExprIdx, Param, ParamIdx, PortIdx, TimeIdx,
};

/// Context used while building the IR.
struct BuildCtx<'a> {
    comp: &'a mut ir::Component,

    // Mapping from names to IR nodes.
    param_map: HashMap<Id, ParamIdx>,
    event_map: HashMap<Id, EventIdx>,
    port_map: HashMap<Id, PortIdx>,
}

impl<'a> BuildCtx<'a> {
    fn new(comp: &'a mut ir::Component) -> Self {
        Self {
            comp,
            param_map: HashMap::new(),
            event_map: HashMap::new(),
            port_map: HashMap::new(),
        }
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
        let idx = self.comp.add(Param::new());
        self.param_map.insert(param, idx);
        idx
    }

    fn time(&mut self, t: ast::Time) -> TimeIdx {
        todo!()
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
                let range = self.range(liveness.take());
                // The bundle type uses a fake bundle index and has a length of 1.
                let live = ir::Liveness {
                    idx: ParamIdx::UNKNOWN,
                    len: self.comp.num(1),
                    range,
                };
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
                let range = self.range(liveness.take());
                let live = ir::Liveness {
                    idx: self.param(*idx),
                    len: self.expr(len.take()),
                    range,
                };
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
}

fn comp(comp: ast::Component) -> ir::Component {
    let mut comp = ir::Component::default();
    todo!()
}
