use super::DisplayCtx;
use crate::{self as ir, Ctx};
use itertools::Itertools;
use std::io;

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
            ir::Command::Let(ir::Let { param, expr }) => {
                write!(f, "{:indent$}let ", "")?;
                self.comp.write(*param, f)?;
                write!(f, " = ")?;
                match expr {
                    Some(expr) => self.comp.write(*expr, f),
                    None => write!(f, "?"),
                }
            }
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
                write!(f, "{:indent$}", "", indent = indent)?;
                if fact.checked {
                    write!(f, "assert ")?;
                } else {
                    write!(f, "assume ")?;
                }
                self.comp.write(fact.prop, f)?;
                write!(f, ";")?;
                if let Some(reason) = self.comp[fact.reason].name() {
                    write!(f, " // {}", reason)?;
                };
                Ok(())
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

    pub fn command_str(&self, c: &ir::Command) -> String {
        let mut buf = Vec::new();
        self.command(c, 0, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
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
        let port_attrs = self.comp.port_attrs.get(idx);
        format!(
            "{:indent$}{}{}: {} {}",
            "",
            port_attrs,
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
        writeln!(f, "{}", self.comp.attrs)?;

        if self.comp.is_ext() {
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
            .param_args()
            .iter()
            .map(|idx| self.comp.display(*idx))
            .join(", ");

        let events = self
            .comp
            .event_args()
            .iter()
            .map(|idx| {
                let ev = self.comp.get(*idx);
                format!(
                    "{}: {}",
                    self.comp.display(*idx),
                    self.comp.display(&ev.delay)
                )
            })
            .join(", ");

        writeln!(f, "[{params}]<{events}>(")?;

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
        writeln!(f, "{outs}) with {{")?;

        for param in self.comp.exist_params() {
            write!(
                f,
                "{:indent$}exists {}",
                "",
                self.comp.display(param),
                indent = indent + 2
            )?;
            if let Some(assumes) = self.comp.get_exist_assumes(param) {
                let props = assumes
                    .iter()
                    .map(|(p, _)| self.comp.display(*p))
                    .join(", ");
                writeln!(f, " where {props};")?;
            } else {
                writeln!(f, ";")?;
            };
        }

        let p_asserts = self.comp.get_param_asserts();
        let e_asserts = self.comp.get_event_asserts();

        if !p_asserts.is_empty() || !e_asserts.is_empty() {
            writeln!(f, "}} where ")?;
            for (idx, _) in p_asserts.iter().chain(e_asserts.iter()) {
                write!(f, "{:indent$}", "", indent = indent + 2)?;
                self.comp.write(*idx, f)?;
                writeln!(f, ",")?;
            }
        }

        writeln!(f, "{:indent$}{{", "")
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
            comp,
            args,
            params,
            lives,
            ..
        } = self.comp.get(idx);
        write!(f, "{:indent$}", "")?;
        self.comp.write(idx, f)?;
        let def_params =
            params.iter().map(|p| self.comp.display(*p)).join(", ");
        if !def_params.is_empty() {
            write!(f, ", {def_params}")?;
        }
        write!(f, " = ")?;

        if let Some(ctx) = self.ctx {
            write!(f, "{}", ctx.display(*comp))?;
        } else {
            write!(f, "{}", comp)?;
        }
        let params = args.iter().map(|p| self.comp.display(*p)).join(", ");
        if !params.is_empty() {
            write!(f, "[{}]", params)?;
        }
        if !lives.is_empty() {
            write!(
                f,
                " in {}",
                lives.iter().map(|l| self.comp.display(l)).join(", ")
            )?;
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
        write!(f, "{:indent$}", "")?;
        self.comp.write(idx, f)?;

        let ports = ports.iter().map(|p| self.comp.display(*p)).join(", ");
        if !ports.is_empty() {
            write!(f, ", {ports}")?;
        }
        write!(f, " = ")?;

        self.comp.write(*inst, f)?;
        write!(
            f,
            "<{}>;",
            events.iter().map(|e| self.comp.display(e.arg)).join(", ")
        )?;

        Ok(())
    }

    pub fn comp(
        &self,
        idx: Option<ir::CompIdx>,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        self.sig(idx, 0, f)?;
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
