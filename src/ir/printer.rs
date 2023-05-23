use crate::ir;
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
    {
        writeln!(f, "{{")?;
        for (i, v) in store.iter() {
            writeln!(f, "{:indent$}{}: {}", "", i, v, indent = indent + 2)?;
        }
        writeln!(f, "{:indent$}}}", "", indent = indent)
    }

    fn index_store<T: Display>(
        store: &ir::IndexStore<T>,
        indent: usize,
        f: &mut impl io::Write,
    ) -> io::Result<()> {
        writeln!(f, "{{")?;
        for (i, v) in store.iter() {
            writeln!(f, "{:indent$}{}: {}", "", i, v, indent = indent + 2)?;
        }
        writeln!(f, "{:indent$}}}", "", indent = indent)
    }

    pub fn comp<F: io::Write>(c: &ir::Component, f: &mut F) -> io::Result<()> {
        let ir::Component {
            exprs,
            times,
            props,
            ports,
            params,
            events,
            instances,
            invocations,
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
        writeln!(f, "}}")
    }
}
