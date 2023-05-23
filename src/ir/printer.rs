use crate::{ir, utils::Idx};
use std::{fmt::Display, io};

pub struct Printer;

impl Printer {
    fn interned<T>(
        store: &ir::Interned<T>,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()>
    where
        T: Eq + std::hash::Hash + Display,
        Idx<T>: Display,
    {
        for (i, v) in store.iter() {
            writeln!(f, "{:indent$}{i} = {v}", "")?;
        }
        Ok(())
    }

    fn index_store<T>(
        store: &ir::IndexStore<T>,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()>
    where
        T: Display,
        Idx<T>: Display,
    {
        for (i, v) in store.iter() {
            writeln!(f, "{:indent$}{i} = {v}", "")?;
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
                write!(f, "{:indent$}{inst}", "")
            }
            ir::Command::Invoke(inv) => {
                write!(f, "{:indent$}{inv}", "")
            }
            ir::Command::Connect(con) => write!(f, "{:indent$}{con}", ""),
            ir::Command::ForLoop(ir::Loop {
                index,
                start,
                end,
                body,
            }) => {
                writeln!(f, "{:indent$}for {index} in {start}..{end} {{", "")?;
                for command in body {
                    Printer::command(command, indent + 2, f)?;
                }
                writeln!(f, "{:indent$}}}", "")
            }
            ir::Command::If(c) => {
                writeln!(f, "{:indent$}if {} {{", "", c.cond)?;
                for command in &c.then {
                    Printer::command(command, indent + 2, f)?;
                }
                writeln!(f, "{:indent$}}}", "")?;
                if !c.alt.is_empty() {
                    writeln!(f, "{:indent$}else {{", "")?;
                    for command in &c.alt {
                        Printer::command(command, indent + 2, f)?;
                    }
                    writeln!(f, "{:indent$}}}", "")?;
                }
                Ok(())
            }
            ir::Command::Fact(fact) => {
                if fact.checked {
                    writeln!(f, "{:indent$}assert {}", "", fact.prop)
                } else {
                    writeln!(f, "{:indent$}assume {}", "", fact.prop)
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
        Printer::interned(exprs, 2, f)?;
        Printer::interned(times, 2, f)?;
        Printer::interned(props, 2, f)?;
        Printer::index_store(ports, 2, f)?;
        Printer::index_store(params, 2, f)?;
        Printer::index_store(events, 2, f)?;
        Printer::index_store(instances, 2, f)?;
        Printer::index_store(invocations, 2, f)?;
        for cmd in cmds {
            Printer::command(cmd, 2, f)?;
        }
        writeln!(f, "}}")
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
