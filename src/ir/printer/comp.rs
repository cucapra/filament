use super::DisplayCtx;
use crate::ir::Ctx;
use crate::{ir, utils::Idx};
use itertools::{Itertools, Position};
use std::{fmt::Display, io};

pub struct Printer<'a> {
    /// The component being printed. Used to resolve interned values like expressions.
    ctx: &'a ir::Component,
}

impl<'a> Printer<'a> {
    pub fn new(ctx: &'a ir::Component) -> Self {
        Self { ctx }
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
        write!(
            f,
            "{:indent$}{} = {};",
            "",
            self.ctx.display(dst),
            self.ctx.display(src),
        )
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
                    self.ctx.display(*index),
                    self.ctx.display(*start),
                    self.ctx.display(*end)
                )?;
                self.commands(body, indent + 2, f)?;
                write!(f, "{:indent$}}}", "")
            }
            ir::Command::If(c) => {
                writeln!(
                    f,
                    "{:indent$}if {} {{",
                    "",
                    self.ctx.display(c.cond)
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
                        self.ctx.display(fact.prop)
                    )
                } else {
                    write!(
                        f,
                        "{:indent$}assume {};",
                        "",
                        self.ctx.display(fact.prop)
                    )
                }
            }
            ir::Command::Exists(ir::Exists { param, expr }) => {
                write!(
                    f,
                    "{:indent$}exists {} = {};",
                    "",
                    self.ctx.display(*param),
                    self.ctx.display(*expr)
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

    fn sig<F: io::Write>(
        &self,
        comp: &ir::Component,
        idx: Option<ir::CompIdx>,
        indent: usize,
        f: &mut F,
    ) -> io::Result<()> {
        if comp.is_ext {
            write!(f, "ext ")?;
        };
        if let Some(info) = &comp.src_info {
            write!(f, "comp {}", info.name)?;
            if log::log_enabled!(log::Level::Debug) {
                if let Some(idx) = idx {
                    write!(f, " {idx}")?;
                }
            }
        } else if let Some(idx) = idx {
            write!(f, "comp {}", idx)?;
        } else {
            write!(f, "comp")?;
        }
        write!(f, "[")?;
        for pos in comp
            .params()
            .iter()
            .filter(|(_, p)| p.is_sig_owned())
            .map(|(idx, _)| idx)
            .with_position()
        {
            match pos {
                Position::First(idx) | Position::Middle(idx) => {
                    write!(f, "{}, ", self.ctx.display(idx))?
                }
                Position::Only(idx) | Position::Last(idx) => {
                    write!(f, "{}", self.ctx.display(idx))?
                }
            }
        }
        write!(f, "]<")?;
        // All events are defined by the signature
        for pos in comp.events().iter().with_position() {
            match pos {
                Position::First((idx, ev)) | Position::Middle((idx, ev)) => {
                    write!(
                        f,
                        "{}: {}, ",
                        self.ctx.display(idx),
                        self.ctx.display(&ev.delay)
                    )?
                }
                Position::Only((idx, ev)) | Position::Last((idx, ev)) => {
                    write!(
                        f,
                        "{}: {}",
                        self.ctx.display(idx),
                        self.ctx.display(&ev.delay)
                    )?
                }
            }
        }
        writeln!(f, ">(")?;
        let print_port = |port: &Position<(ir::PortIdx, &ir::Port)>,
                          f: &mut F| match port {
            Position::First((idx, port)) | Position::Middle((idx, port)) => {
                writeln!(
                    f,
                    "{:indent$}{}: {} {},",
                    "",
                    self.ctx.display(*idx),
                    self.ctx.display(&port.live),
                    self.ctx.display(port.width),
                    indent = indent + 2
                )
            }
            Position::Only((idx, port)) | Position::Last((idx, port)) => {
                writeln!(
                    f,
                    "{:indent$}{}: {} {}",
                    "",
                    self.ctx.display(*idx),
                    self.ctx.display(&port.live),
                    self.ctx.display(port.width),
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
        &self,
        idx: ir::ParamIdx,
        indent: usize,
        c: &ir::Component,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let param = self.ctx.get(idx);
        match param.owner {
            ir::ParamOwner::Sig | ir::ParamOwner::Instance(_) => {}
            ir::ParamOwner::Bundle(_)
            | ir::ParamOwner::Loop
            | ir::ParamOwner::Exists => {
                writeln!(
                    f,
                    "{:indent$}{idx} = param {param};{comment}",
                    "",
                    param = self.ctx.display(idx),
                    comment = c
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
        } = self.ctx.get(idx);
        match &owner {
            ir::PortOwner::Sig { .. } => Ok(()),
            ir::PortOwner::Inv { dir, .. } => {
                if log::log_enabled!(log::Level::Debug) {
                    write!(
                        f,
                        "{:indent$}{} ({idx}): bundle({dir}) {} {};",
                        "",
                        self.ctx.display(idx),
                        self.ctx.display(live),
                        self.ctx.display(*width),
                    )
                } else {
                    write!(
                        f,
                        "{:indent$}{}: bundle({dir}) {} {};",
                        "",
                        self.ctx.display(idx),
                        self.ctx.display(live),
                        self.ctx.display(*width),
                    )
                }
            }
            ir::PortOwner::Local => {
                if log::log_enabled!(log::Level::Debug) {
                    write!(
                        f,
                        "{:indent$}{} ({idx}) = bundle {} {};",
                        "",
                        self.ctx.display(idx),
                        self.ctx.display(live),
                        self.ctx.display(*width),
                    )
                } else {
                    write!(
                        f,
                        "{:indent$}{} = bundle {} {};",
                        "",
                        self.ctx.display(idx),
                        self.ctx.display(live),
                        self.ctx.display(*width),
                    )
                }
            }
        }
    }

    pub fn instance(
        &self,
        idx: ir::InstIdx,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        write!(f, "{:indent$}{} = instance ", "", self.ctx.display(idx))?;
        let ir::Instance { comp, params, .. } = self.ctx.get(idx);
        write!(f, "{}[", comp)?;
        for (i, param) in params.iter().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", self.ctx.display(*param))?;
        }
        write!(f, "];")
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
        } = self.ctx.get(idx);

        write!(
            f,
            "{:indent$}{inv}, {ports} = invoke {inst}<{events}>;",
            "",
            inv = self.ctx.display(idx),
            ports = ports.iter().map(|p| self.ctx.display(*p)).join(", "),
            inst = self.ctx.display(*inst),
            events = events.iter().map(|e| self.ctx.display(e.arg)).join(", ")
        )?;

        Ok(())
    }

    pub fn comp(
        ctx: &ir::Component,
        idx: Option<ir::CompIdx>,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        let printer = ir::Printer { ctx };
        printer.sig(ctx, idx, 0, f)?;
        // If debugging is enabled, show the low-level representation of the
        // component's interned values and other stores.
        if log::log_enabled!(log::Level::Debug) {
            for idx in ctx.params().idx_iter() {
                printer.local_param(idx, 2, ctx, f)?;
            }
            Printer::interned(ctx.exprs(), "expr", 2, f)?;
            Printer::interned(ctx.times(), "time", 2, f)?;
            Printer::interned(ctx.props(), "prop", 2, f)?;
            writeln!(f, "control:")?;
        }
        printer.commands(&ctx.cmds, 2, f)?;
        writeln!(f, "}}")
    }

    /// Get a string representation of a component
    pub fn comp_str(c: &ir::Component) -> String {
        let mut buf = Vec::new();
        Printer::comp(c, None, &mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    pub fn context(
        ctx: &ir::Context,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for (idx, comp) in ctx.comps.iter() {
            Printer::comp(comp, Some(idx), f)?
        }
        Ok(())
    }
}
