use super::Ctx;
use crate::{ast, ir, utils::Idx};
use itertools::{Itertools, Position};
use std::{fmt::Display, io};

/// A context capable of displaying [Idx<T>] values.
pub trait DisplayCtx<T>
where
    Self: Ctx<T>,
{
    /// Display the value
    fn display(&self, idx: Idx<T>) -> String;
}

impl DisplayCtx<ir::Expr> for ir::Component {
    fn display(&self, idx: Idx<ir::Expr>) -> String {
        self.display_expr_helper(idx, ECtx::default())
    }
}

impl DisplayCtx<ir::Prop> for ir::Component {
    fn display(&self, idx: Idx<ir::Prop>) -> String {
        self.display_prop_helper(idx, PCtx::Implies)
    }
}

impl DisplayCtx<ir::Time> for ir::Component {
    fn display(&self, idx: Idx<ir::Time>) -> String {
        let &ir::Time { event, offset } = self.get(idx);
        if let Some(off) = offset.as_concrete(self) {
            if off == 0 {
                return self.display(event);
            }
        }
        format!("{}+{}", self.display(event), self.display(offset))
    }
}

impl DisplayCtx<ir::Event> for ir::Component {
    fn display(&self, idx: Idx<ir::Event>) -> String {
        if log::log_enabled!(log::Level::Debug) {
            format!("{idx}")
        } else {
            let ev = self.get(idx);
            let ir::Info::Event { name, .. } = self.get(ev.info) else {
                unreachable!("Expccted event info")
            };
            match ev.owner {
                ir::EventOwner::Sig => name.to_string(),
                ir::EventOwner::Inv { inv } => {
                    format!("{}.{name}", inv)
                }
            }
        }
    }
}

impl DisplayCtx<ir::Param> for ir::Component {
    fn display(&self, idx: ir::ParamIdx) -> String {
        if log::log_enabled!(log::Level::Debug) {
            format!("{idx}")
        } else {
            let param = self.get(idx);
            let ir::Info::Param { name, .. } = self.get(param.info) else {
                unreachable!("Expected param info");
            };
            format!("#{name}")
        }
    }
}

impl DisplayCtx<ir::Port> for ir::Component {
    fn display(&self, idx: ir::PortIdx) -> String {
        let port = self.get(idx);
        let ir::Info::Port { name, .. } = self.get(port.info) else {
            unreachable!("Expected port info")
        };
        name.to_string()
    }
}

pub struct Printer<'a> {
    /// The component being printed. Used to resolve interned values like expressions.
    ctx: &'a ir::Component,
}

impl Printer<'_> {
    fn interned<T>(
        store: &ir::Interned<T>,
        op: &str,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()>
    where
        T: Eq + std::hash::Hash + Display,
        Idx<T>: Display,
    {
        for (i, v) in store.iter() {
            writeln!(f, "{:indent$}{i} = {op} {v};", "")?;
        }
        Ok(())
    }

    fn expr(&self, e: ir::ExprIdx) -> String {
        self.ctx.display(e)
    }

    fn time(&self, t: ir::TimeIdx) -> String {
        self.ctx.display(t)
    }

    fn range(&self, r: &ir::Range) -> String {
        let ir::Range { start, end } = r;
        format!("@[{}, {}]", self.time(*start), self.time(*end))
    }

    fn liveness(&self, l: &ir::Liveness) -> String {
        let ir::Liveness { idx, len, range } = l;
        format!("for<{idx}: {}> {}", self.expr(*len), self.range(range))
    }

    fn commands(
        &self,
        cmds: &[ir::Command],
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for cmd in cmds {
            self.command(cmd, indent, f)?;
            writeln!(f)?;
        }
        Ok(())
    }

    fn access(&self, a: &ir::Access) -> String {
        let &ir::Access { port, start, end } = a;
        format!("{port}[{}..{})", self.expr(start), self.expr(end))
    }

    fn connect(
        &self,
        c: &ir::Connect,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Connect { src, dst, .. } = c;
        write!(
            f,
            "{:indent$}{} = {};",
            "",
            self.access(dst),
            self.access(src),
        )
    }

    pub fn command(
        &self,
        c: &ir::Command,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        match c {
            ir::Command::Instance(inst) => {
                write!(f, "{:indent$}{inst};", "")
            }
            ir::Command::Invoke(inv) => {
                write!(f, "{:indent$}{inv};", "")
            }
            ir::Command::Connect(con) => self.connect(con, indent, f),
            ir::Command::EventBind(ir::EventBind { event, arg, .. }) => {
                write!(f, "{:indent$}bind {} to {}", "", event, self.time(*arg))
            }
            ir::Command::ForLoop(ir::Loop {
                index,
                start,
                end,
                body,
            }) => {
                writeln!(
                    f,
                    "{:indent$}for {index} in {}..{} {{",
                    "",
                    self.expr(*start),
                    self.expr(*end)
                )?;
                self.commands(body, indent + 2, f)?;
                write!(f, "{:indent$}}}", "")
            }
            ir::Command::If(c) => {
                writeln!(f, "{:indent$}if {} {{", "", c.cond)?;
                self.commands(&c.then, indent + 2, f)?;
                writeln!(f, "{:indent$}}}", "")?;
                if !c.alt.is_empty() {
                    writeln!(f, "{:indent$}else {{", "")?;
                    self.commands(&c.alt, indent + 2, f)?;
                    writeln!(f, "{:indent$}}}", "")?;
                }
                Ok(())
            }
            ir::Command::Fact(fact) => {
                if fact.checked {
                    write!(f, "{:indent$}assert {};", "", fact.prop)
                } else {
                    write!(f, "{:indent$}assume {};", "", fact.prop)
                }
            }
        }
    }

    fn sig<F: io::Write>(
        &self,
        comp: &ir::Component,
        indent: usize,
        f: &mut F,
    ) -> io::Result<()> {
        if comp.is_ext {
            write!(f, "ext ")?;
        }
        write!(f, "comp {}[", comp.idx())?;
        for pos in comp
            .params()
            .iter()
            .filter(|(_, p)| p.is_sig_owned())
            .map(|(idx, _)| idx)
            .with_position()
        {
            match pos {
                Position::First(idx) | Position::Middle(idx) => {
                    write!(f, "{idx}, ")?
                }
                Position::Only(idx) | Position::Last(idx) => {
                    write!(f, "{idx}")?
                }
            }
        }
        write!(f, "]<")?;
        // All events are defined by the signature
        for pos in comp
            .events()
            .iter()
            .filter(|(_, eb)| matches!(eb.owner, ir::EventOwner::Sig))
            .with_position()
        {
            match pos {
                Position::First((idx, ev)) | Position::Middle((idx, ev)) => {
                    write!(
                        f,
                        "{idx}: {}, ",
                        self.ctx.display_timesub(&ev.delay)
                    )?
                }
                Position::Only((idx, ev)) | Position::Last((idx, ev)) => {
                    write!(f, "{idx}: {}", self.ctx.display_timesub(&ev.delay))?
                }
            }
        }
        writeln!(f, ">(")?;
        let print_port = |port: &Position<(ir::PortIdx, &ir::Port)>,
                          f: &mut F| match port {
            Position::First((idx, port)) | Position::Middle((idx, port)) => {
                writeln!(
                    f,
                    "{:indent$}{idx}: {} {},",
                    "",
                    self.liveness(&port.live),
                    self.expr(port.width),
                    indent = indent + 2
                )
            }
            Position::Only((idx, port)) | Position::Last((idx, port)) => {
                writeln!(
                    f,
                    "{:indent$}{idx}: {} {}",
                    "",
                    self.liveness(&port.live),
                    self.expr(port.width),
                    indent = indent + 2
                )
            }
        };
        // Print input ports first. The direction is reversed when they are
        // bound in the body.
        for pos in comp
            .ports()
            .iter()
            .filter(|(_, port)| port.is_sig_in())
            .with_position()
        {
            print_port(&pos, f)?;
        }

        writeln!(f, ") -> (")?;
        for pos in comp
            .ports()
            .iter()
            .filter(|(_, port)| port.is_sig_out())
            .with_position()
        {
            print_port(&pos, f)?;
        }
        writeln!(f, ") {{")
    }

    fn local_param(
        idx: ir::ParamIdx,
        param: &ir::Param,
        indent: usize,
        c: &ir::Component,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        if !param.is_sig_owned() {
            let &ir::Param { info, .. } = c.get(idx);
            let ir::Info::Param { name, .. } = c.get(info) else {
                unreachable!("Expected param info");
            };
            writeln!(f, "{:indent$}{idx} = param {param}; // {name}", "",)?;
        }
        Ok(())
    }

    fn local_port(
        &self,
        idx: ir::PortIdx,
        port: &ir::Port,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Port {
            owner, width, live, ..
        } = port;
        match &owner {
            ir::PortOwner::Sig { .. } => Ok(()),
            ir::PortOwner::Inv { dir, .. } => {
                writeln!(
                    f,
                    "{:indent$}{idx}: bundle({dir}) {} {};",
                    "",
                    self.liveness(live),
                    self.expr(*width),
                )
            }
            ir::PortOwner::Local => {
                writeln!(
                    f,
                    "{:indent$}{idx} = bundle {} {};",
                    "",
                    self.liveness(live),
                    self.expr(*width),
                )
            }
        }
    }

    fn event(
        ctx: &ir::Component,
        idx: ir::EventIdx,
        event: &ir::Event,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Event { owner, delay, .. } = event;
        match owner {
            ir::EventOwner::Sig => Ok(()),
            ir::EventOwner::Inv { inv } => {
                writeln!(
                    f,
                    "{:indent$}{idx} = event({inv}), delay: {};",
                    "",
                    ctx.display_timesub(delay)
                )
            }
        }
    }

    pub fn instance(
        ctx: &ir::Component,
        idx: ir::InstIdx,
        inst: &ir::Instance,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        write!(f, "{:indent$}{idx} = instance ", "")?;
        let ir::Instance { comp, params } = inst;
        write!(f, "{}[", comp)?;
        for (i, param) in params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", ctx.display(*param))?;
        }
        writeln!(f, "];")
    }

    pub fn invoke(
        idx: ir::InvIdx,
        c: &ir::Invoke,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Invoke { inst, ports } = c;

        writeln!(
            f,
            "{:indent$}{idx}, {ports} = invoke {inst};",
            "",
            ports = ports.iter().map(|p| format!("{p}")).join(", "),
        )?;

        Ok(())
    }

    pub fn comp(ctx: &ir::Component, f: &mut impl io::Write) -> io::Result<()> {
        let printer = ir::Printer { ctx };
        printer.sig(ctx, 0, f)?;
        for (idx, param) in ctx.params().iter() {
            Printer::local_param(idx, param, 2, ctx, f)?;
        }
        for (idx, event) in ctx.events().iter() {
            Printer::event(ctx, idx, event, 2, f)?;
        }
        // If debugging is enabled, show the low-level representation of the
        // component's interned values.
        if log::log_enabled!(log::Level::Debug) {
            Printer::interned(ctx.exprs(), "expr", 2, f)?;
            Printer::interned(ctx.times(), "time", 2, f)?;
            Printer::interned(ctx.props(), "prop", 2, f)?;
        }
        for (idx, port) in ctx.ports().iter() {
            printer.local_port(idx, port, 2, f)?;
        }
        for (idx, instance) in ctx.instances().iter() {
            Printer::instance(ctx, idx, instance, 2, f)?;
        }
        for (idx, invoke) in ctx.invocations().iter() {
            Printer::invoke(idx, invoke, 2, f)?;
        }
        writeln!(f, "control:")?;
        printer.commands(&ctx.cmds, 2, f)?;
        writeln!(f, "}}")
    }

    /// Get a string representation of a component
    pub fn comp_str(c: &ir::Component) -> String {
        let mut buf = Vec::new();
        Printer::comp(c, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    pub fn context(
        ctx: &ir::Context,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for (_, comp) in ctx.comps.iter() {
            Printer::comp(comp, f)?
        }
        Ok(())
    }
}

// ========= Pretty printing for expressions and propositions =========

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
/// Track the current context within an expression for pretty printing
enum ECtx {
    #[default]
    /// Inside an addition priority expression (+ or -)
    Add,
    /// Substraction priority expression (-)
    Sub,
    /// Inside an multiplication priority expression (* or / or %)
    Mul,
}

impl From<ast::Op> for ECtx {
    fn from(op: ast::Op) -> Self {
        match op {
            ast::Op::Add => ECtx::Add,
            ast::Op::Sub => ECtx::Sub,
            ast::Op::Mul | ast::Op::Div | ast::Op::Mod => ECtx::Mul,
        }
    }
}

// Ordering for expression printing context.
// If `self > other`, then that means that `self` binds tigher than `other`.
impl Ord for ECtx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        match (self, other) {
            // Mults are next
            (ECtx::Mul, ECtx::Mul) => Equal,
            (ECtx::Mul, _) => Greater,
            // Subs are next
            (ECtx::Sub, ECtx::Sub) => Equal,
            (ECtx::Sub, ECtx::Mul) => Less,
            (ECtx::Sub, _) => Greater,
            // Adds are last
            (ECtx::Add, ECtx::Add) => Equal,
            (ECtx::Add, _) => Less,
        }
    }
}

impl PartialOrd for ECtx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// Context to track proposition bindings
enum PCtx {
    Not,
    Cmp,
    And,
    Or,
    Implies,
}

impl Ord for PCtx {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use std::cmp::Ordering::*;
        use PCtx::*;
        match (self, other) {
            // Negations
            (Not, Not) => Equal,
            (Not, _) => Greater,
            // Comparisons
            (Cmp, Cmp) => Equal,
            (Cmp, Not) => Less,
            (Cmp, _) => Greater,
            // Conjunctions
            (And, And) => Equal,
            (And, Not | Cmp) => Less,
            (And, _) => Greater,
            // Disjunctions
            (Or, Or) => Equal,
            (Or, Not | And | Cmp) => Less,
            (Or, _) => Greater,
            // Implications
            (Implies, Implies) => Equal,
            (Implies, _) => Less,
        }
    }
}

impl PartialOrd for PCtx {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Implement methods to display various values bound by the component
impl ir::Component {
    fn display_expr_helper(&self, expr: ir::ExprIdx, ctx: ECtx) -> String {
        match self.get(expr) {
            ir::Expr::Param(p) => self.display(*p),
            ir::Expr::Concrete(n) => format!("{n}"),
            ir::Expr::Bin { op, lhs, rhs } => {
                let inner = ECtx::from(*op);
                let left = self.display_expr_helper(*lhs, inner);
                let right = self.display_expr_helper(*rhs, inner);
                // If context binds more tightly than the inner operator,
                // wrap the inner expression in parens.
                if ctx > inner {
                    format!("({}{}{})", left, op, right)
                } else {
                    format!("{}{}{}", left, op, right)
                }
            }
            ir::Expr::Fn { op, args } => {
                let fn_str = match op {
                    ast::UnFn::Pow2 => "pow2",
                    ast::UnFn::Log2 => "log2",
                };
                format!(
                    "{fn_str}({args})",
                    args = args
                        .iter()
                        .map(|a| self.display_expr_helper(*a, ECtx::default()))
                        .join(", ")
                )
            }
        }
    }

    fn display_cmp<T>(
        &self,
        cmp: &ir::CmpOp<T>,
        ctx: PCtx,
        print_base: impl Fn(T) -> String,
    ) -> String
    where
        T: Clone,
    {
        let ir::CmpOp { op, lhs, rhs } = cmp;
        let l = print_base(lhs.clone());
        let r = print_base(rhs.clone());
        if ctx > PCtx::Cmp {
            format!("({} {} {})", l, op, r)
        } else {
            format!("{} {} {}", l, op, r)
        }
    }

    fn display_prop_helper(&self, prop: ir::PropIdx, ctx: PCtx) -> String {
        match self.get(prop) {
            ir::Prop::True => "true".to_string(),
            ir::Prop::False => "false".to_string(),
            ir::Prop::Cmp(c) => self.display_cmp(c, ctx, |e| self.display(e)),
            ir::Prop::TimeCmp(cmp) => {
                self.display_cmp(cmp, ctx, |t| self.display(t))
            }
            ir::Prop::TimeSubCmp(cmp) => {
                self.display_cmp(cmp, ctx, |t| self.display_timesub(&t))
            }
            ir::Prop::Not(p) => {
                format!("!{}", self.display_prop_helper(*p, PCtx::Not))
            }
            ir::Prop::And(l, r) => {
                let inner = PCtx::And;
                let l = self.display_prop_helper(*l, inner);
                let r = self.display_prop_helper(*r, inner);
                if inner < ctx {
                    format!("({} & {})", l, r)
                } else {
                    format!("{} & {}", l, r)
                }
            }
            ir::Prop::Or(l, r) => {
                let inner = PCtx::Or;
                let l = self.display_prop_helper(*l, inner);
                let r = self.display_prop_helper(*r, inner);
                if inner < ctx {
                    format!("({} | {})", l, r)
                } else {
                    format!("{} | {}", l, r)
                }
            }
            ir::Prop::Implies(l, r) => {
                let inner = PCtx::Implies;
                let l = self.display_prop_helper(*l, inner);
                let r = self.display_prop_helper(*r, inner);
                if inner < ctx {
                    format!("({} => {})", l, r)
                } else {
                    format!("{} => {}", l, r)
                }
            }
        }
    }

    /// Display a [super::TimeSub] expression in surface-level syntax
    pub fn display_timesub(&self, ts: &ir::TimeSub) -> String {
        match ts {
            ir::TimeSub::Unit(e) => self.display(*e),
            ir::TimeSub::Sym { l, r } => {
                format!("|{} - {}|", self.display(*l), self.display(*r))
            }
        }
    }

    /// Surface-level visualization for a range
    pub fn display_range(&self, r: &ir::Range) -> String {
        format!("@[{}, {}]", self.display(r.start), self.display(r.end))
    }
}
