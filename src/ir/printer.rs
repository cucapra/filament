use crate::{ir, utils::Idx};
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

    pub fn comp(c: &ir::Component, f: &mut impl io::Write) -> io::Result<()> {
        let ir::Component {
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
        writeln!(f, "component {{")?;
        Printer::index_store(params, "param", 2, f)?;
        Printer::interned(exprs, "expr", 2, f)?;
        Printer::interned(times, "time", 2, f)?;
        Printer::interned(props, "prop", 2, f)?;
        Printer::index_store(ports, "port", 2, f)?;
        Printer::index_store(events, "event", 2, f)?;
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
