use super::Ctx;
use crate::{ir, utils::Idx};
use itertools::{Itertools, Position};
use std::{fmt::Display, io};

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

    fn index_store<T>(
        store: &ir::IndexStore<T>,
        op: &str,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()>
    where
        T: Display,
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
            ir::Command::Connect(con) => write!(f, "{:indent$}{con}", ""),
            ir::Command::EventBind(ir::EventBind { event, arg }) => {
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
        comp: ir::CompIdx,
        params: &ir::IndexStore<ir::Param>,
        events: &ir::IndexStore<ir::Event>,
        ports: &ir::IndexStore<ir::Port>,
        indent: usize,
        f: &mut F,
    ) -> io::Result<()> {
        write!(f, "comp {comp}[")?;
        for pos in params
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
        for pos in events
            .iter()
            .filter(|(_, eb)| matches!(eb.owner, ir::EventOwner::Sig))
            .with_position()
        {
            match pos {
                Position::First((idx, ev)) | Position::Middle((idx, ev)) => {
                    write!(f, "{idx}: {}, ", ev.delay)?
                }
                Position::Only((idx, ev)) | Position::Last((idx, ev)) => {
                    write!(f, "{idx}: {}", ev.delay)?
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
        for pos in ports
            .iter()
            .filter(|(_, port)| port.is_sig_in())
            .with_position()
        {
            print_port(&pos, f)?;
        }

        writeln!(f, ") -> (")?;
        for pos in ports
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
                    "{:indent$}{idx} = event({inv}), delay: {delay};",
                    "",
                )
            }
        }
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

    pub fn comp(c: &ir::Component, f: &mut impl io::Write) -> io::Result<()> {
        let printer = ir::Printer { ctx: c };
        let ir::Component {
            idx,
            exprs,
            times,
            props,
            ports,
            params,
            events,
            instances,
            invocations,
            cmds,
            info: _,
        } = &c;
        printer.sig(*idx, params, events, ports, 0, f)?;
        for (idx, param) in params.iter() {
            Printer::local_param(idx, param, 2, c, f)?;
        }
        // Printer::interned(exprs, "expr", 2, f)?;
        for (idx, event) in events.iter() {
            Printer::event(idx, event, 2, f)?;
        }
        // Printer::interned(times, "time", 2, f)?;
        // Printer::interned(props, "prop", 2, f)?;
        for (idx, port) in ports.iter() {
            printer.local_port(idx, port, 2, f)?;
        }
        Printer::index_store(instances, "instance", 2, f)?;
        for (idx, invoke) in invocations.iter() {
            Printer::invoke(idx, invoke, 2, f)?;
        }
        writeln!(f, "control:")?;
        printer.commands(cmds, 2, f)?;
        writeln!(f, "}}")
    }

    pub fn external(
        ext: &ir::External,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        writeln!(f, "external comp {};", ext.idx)
    }

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
            match comp {
                ir::CompOrExt::Comp(comp) => Printer::comp(comp, f)?,
                ir::CompOrExt::Ext(ext) => Printer::external(ext, f)?,
            }
        }
        Ok(())
    }
}
