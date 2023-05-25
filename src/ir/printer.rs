use crate::{ir, utils::Idx};
use itertools::{Itertools, Position};
use std::{fmt::Display, io};

pub struct Printer;

impl Printer {
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

    fn commands(
        cmds: &[ir::Command],
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for cmd in cmds {
            Printer::command(cmd, indent, f)?;
            writeln!(f)?;
        }
        Ok(())
    }

    pub fn command(
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
            ir::Command::ForLoop(ir::Loop {
                index,
                start,
                end,
                body,
            }) => {
                writeln!(f, "{:indent$}for {index} in {start}..{end} {{", "")?;
                Printer::commands(body, indent + 2, f)?;
                write!(f, "{:indent$}}}", "")
            }
            ir::Command::If(c) => {
                writeln!(f, "{:indent$}if {} {{", "", c.cond)?;
                Printer::commands(&c.then, indent + 2, f)?;
                writeln!(f, "{:indent$}}}", "")?;
                if !c.alt.is_empty() {
                    writeln!(f, "{:indent$}else {{", "")?;
                    Printer::commands(&c.alt, indent + 2, f)?;
                    writeln!(f, "{:indent$}}}", "")?;
                }
                Ok(())
            }
            ir::Command::Fact(fact) => {
                if fact.checked {
                    writeln!(f, "{:indent$}assert {}", "", fact.prop)
                } else {
                    writeln!(f, "{:indent$}assume {}ctx", "", fact.prop)
                }
            }
        }
    }

    fn sig<F: io::Write>(
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
        for pos in events.iter().with_position() {
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
                    port.live,
                    port.width,
                    indent = indent + 2
                )
            }
            Position::Only((idx, port)) | Position::Last((idx, port)) => {
                writeln!(
                    f,
                    "{:indent$}{idx}: {} {}",
                    "",
                    port.live,
                    port.width,
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

    fn print_local_params(
        params: &ir::IndexStore<ir::Param>,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for (idx, param) in params.iter() {
            if !param.is_sig_owned() {
                writeln!(f, "{:indent$}{idx} = param {param};", "",)?;
            }
        }
        Ok(())
    }

    fn print_local_ports(
        ports: &ir::IndexStore<ir::Port>,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        for (idx, port) in ports.iter() {
            let ir::Port { owner, width, live } = port;
            match &owner {
                ir::PortOwner::Sig { .. } => continue,
                ir::PortOwner::Inv { inv, dir } => {
                    writeln!(
                        f,
                        "{:indent$}{inv}.{idx} = bundle({dir}) {live} {width};",
                        "",
                    )?;
                }
                ir::PortOwner::Local => {
                    writeln!(
                        f,
                        "{:indent$}{idx} = bundle {live} {width};",
                        "",
                    )?;
                }
            }
        }
        Ok(())
    }

    pub fn comp(c: &ir::Component, f: &mut impl io::Write) -> io::Result<()> {
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
        } = &c;
        Printer::sig(*idx, params, events, ports, 0, f)?;
        Printer::print_local_params(params, 2, f)?;
        Printer::interned(exprs, "expr", 2, f)?;
        Printer::interned(times, "time", 2, f)?;
        Printer::interned(props, "prop", 2, f)?;
        Printer::print_local_ports(ports, 2, f)?;
        Printer::index_store(instances, "instance", 2, f)?;
        Printer::index_store(invocations, "invoke", 2, f)?;
        writeln!(f, "control:")?;
        Printer::commands(cmds, 2, f)?;
        writeln!(f, "}}")
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
        for comp in &ctx.comps {
            Printer::comp(comp, f)?;
        }
        Ok(())
    }
}
