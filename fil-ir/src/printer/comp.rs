use super::DisplayCtx;
use crate::{self as ir, Ctx, Idx};
use itertools::Itertools;
use std::{fmt::Display, io};

pub struct Printer<'a, 'ctx: 'a> {
    /// The component being printed. Used to resolve interned values like expressions.
    comp: &'a ir::Component,
    ctx: Option<&'ctx ir::Context>,
}

impl<'a, 'b> Printer<'a, 'b> {
    pub fn new(comp: &'a ir::Component) -> Self {
        Self { comp, ctx: None }
    }

    pub fn with_ctx(mut self, ctx: &'b ir::Context) -> Self {
        self.ctx = Some(ctx);
        self
    }

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

    fn connect(
        &self,
        c: &ir::Connect,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Connect { src, dst, .. } = c;
        write!(f, "{:indent$}", "")?;
        self.comp.write(dst, f)?;
        write!(f, " = ")?;
        self.comp.write(src, f)?;
        write!(f, ";")
    }

    pub fn command(
        &self,
        c: &ir::Command,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        match c {
            ir::Command::Instance(inst) => self.instance(*inst, indent, f),
            ir::Command::Invoke(inv) => self.invoke(*inv, indent, f),
            ir::Command::Connect(con) => self.connect(con, indent, f),
            ir::Command::BundleDef(p) => self.local_port(*p, indent, f),
            ir::Command::ForLoop(ir::Loop {
                index,
                start,
                end,
                body,
            }) => {
                writeln!(
                    f,
                    "{:indent$}for {} in {}..{} {{",
                    "",
                    self.comp.display(*index),
                    self.comp.display(*start),
                    self.comp.display(*end)
                )?;
                self.commands(body, indent + 2, f)?;
                write!(f, "{:indent$}}}", "")
            }
            ir::Command::If(c) => {
                writeln!(
                    f,
                    "{:indent$}if {} {{",
                    "",
                    self.comp.display(c.cond)
                )?;
                self.commands(&c.then, indent + 2, f)?;
                write!(f, "{:indent$}}}", "")?;
                if !c.alt.is_empty() {
                    writeln!(f, " else {{")?;
                    self.commands(&c.alt, indent + 2, f)?;
                    write!(f, "{:indent$}}}", "")?;
                }
                Ok(())
            }
            ir::Command::Fact(fact) => {
                if fact.checked {
                    write!(
                        f,
                        "{:indent$}assert {};",
                        "",
                        self.comp.display(fact.prop)
                    )
                } else {
                    write!(
                        f,
                        "{:indent$}assume {};",
                        "",
                        self.comp.display(fact.prop)
                    )
                }
            }
            ir::Command::Exists(ir::Exists { param, expr }) => {
                write!(
                    f,
                    "{:indent$}exists {} = {};",
                    "",
                    self.comp.display(*param),
                    self.comp.display(*expr)
                )
            }
        }
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

    fn port(&self, idx: ir::PortIdx, indent: usize) -> String {
        let port = self.comp.get(idx);
        format!(
            "{:indent$}{}: {} {}",
            "",
            self.comp.display(idx),
            self.comp.display(&port.live),
            self.comp.display(port.width),
            indent = indent + 2
        )
    }

    fn sig<F: io::Write>(
        &self,
        idx: Option<ir::CompIdx>,
        indent: usize,
        f: &mut F,
    ) -> io::Result<()> {
        if self.comp.is_ext {
            write!(f, "ext ")?;
        };
        if let Some(info) = &self.comp.src_info {
            write!(f, "comp {}", info.name)?;
        } else if let Some(idx) = idx {
            write!(f, "comp {}", idx)?;
        } else {
            write!(f, "comp")?;
        }

        let params = self
            .comp
            .params()
            .iter()
            .filter(|(_, p)| p.is_sig_owned())
            .map(|(idx, _)| self.comp.display(idx))
            .join(", ");

        let events = self
            .comp
            .events()
            .iter()
            .map(|(idx, ev)| {
                format!(
                    "{}: {}",
                    self.comp.display(idx),
                    self.comp.display(&ev.delay)
                )
            })
            .join(",");

        writeln!(f, "[{params}]]<{events}>(")?;

        // Print input ports first. The direction is reversed when they are
        // bound in the body.
        let inps = self
            .comp
            .ports()
            .iter()
            .filter(|(_, port)| port.is_sig_in())
            .map(|(idx, _)| self.port(idx, indent))
            .join(",\n");
        writeln!(f, "{inps}) -> (")?;

        let outs = self
            .comp
            .ports()
            .iter()
            .filter(|(_, port)| port.is_sig_out())
            .map(|(idx, _)| self.port(idx, indent))
            .join(",\n");

        writeln!(f, "{outs}) {{")
    }

    fn local_param(
        &self,
        idx: ir::ParamIdx,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let param = self.comp.get(idx);
        match param.owner {
            ir::ParamOwner::Sig | ir::ParamOwner::Instance { .. } => {}
            ir::ParamOwner::Bundle(_)
            | ir::ParamOwner::Loop
            | ir::ParamOwner::Exists => {
                writeln!(
                    f,
                    "{:indent$}{idx} = param {param};{comment}",
                    "",
                    param = self.comp.display(idx),
                    comment = self
                        .comp
                        .get(param.info)
                        .as_param()
                        .map_or("".to_string(), |p| format!(" // {}", p.name))
                )?;
            }
        }

        Ok(())
    }

    fn local_port(
        &self,
        idx: ir::PortIdx,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Port {
            owner, width, live, ..
        } = self.comp.get(idx);
        match &owner {
            ir::PortOwner::Sig { .. } => Ok(()),
            ir::PortOwner::Inv { dir, .. } => {
                write!(
                    f,
                    "{:indent$}{}: bundle({dir}) {} {};",
                    "",
                    self.comp.display(idx),
                    self.comp.display(live),
                    self.comp.display(*width),
                )
            }
            ir::PortOwner::Local => {
                write!(
                    f,
                    "{:indent$}{} = bundle {} {};",
                    "",
                    self.comp.display(idx),
                    self.comp.display(live),
                    self.comp.display(*width),
                )
            }
        }
    }

    pub fn instance(
        &self,
        idx: ir::InstIdx,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Instance {
            comp, args, params, ..
        } = self.comp.get(idx);
        let def_params =
            params.iter().map(|p| self.comp.display(*p)).join(", ");
        write!(
            f,
            "{:indent$}{}, {} = instance ",
            "",
            self.comp.display(idx),
            def_params,
        )?;
        if let Some(ctx) = self.ctx {
            write!(f, "{}", ctx.display(*comp))?;
        } else {
            write!(f, "{}", comp)?;
        }
        let params = args.iter().map(|p| self.comp.display(*p)).join(", ");
        if !params.is_empty() {
            write!(f, "[{}]", params)?;
        }
        write!(f, ";")
    }

    pub fn invoke(
        &self,
        idx: ir::InvIdx,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let ir::Invoke {
            inst,
            ports,
            events,
            ..
        } = self.comp.get(idx);

        write!(
            f,
            "{:indent$}{inv}, {ports} = invoke {inst}<{events}>;",
            "",
            inv = self.comp.display(idx),
            ports = ports.iter().map(|p| self.comp.display(*p)).join(", "),
            inst = self.comp.display(*inst),
            events = events.iter().map(|e| self.comp.display(e.arg)).join(", ")
        )?;

        Ok(())
    }

    pub fn comp(
        &self,
        idx: Option<ir::CompIdx>,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        self.sig(idx, 0, f)?;
        // If debugging is enabled, show the low-level representation of the
        // component's interned values and other stores.
        if log::log_enabled!(log::Level::Trace) {
            for idx in self.comp.params().idx_iter() {
                self.local_param(idx, 2, f)?;
            }
            Printer::interned(self.comp.exprs(), "expr", 2, f)?;
            Printer::interned(self.comp.times(), "time", 2, f)?;
            Printer::interned(self.comp.props(), "prop", 2, f)?;
            writeln!(f, "control:")?;
        }
        self.commands(&self.comp.cmds, 2, f)?;
        writeln!(f, "}}")
    }

    /// Get a string representation of a component
    pub fn comp_str(c: &ir::Component) -> String {
        let printer = Printer::new(c);
        let mut buf = Vec::new();
        printer.comp(None, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    pub fn context(
        ctx: &ir::Context,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for (idx, comp) in ctx.comps.iter() {
            Printer::new(comp).with_ctx(ctx).comp(Some(idx), f)?
        }
        Ok(())
    }
}
