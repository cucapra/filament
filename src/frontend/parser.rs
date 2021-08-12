#![allow(clippy::upper_case_acronyms)]

//! Parser for Calyx programs.
use crate::errors::{self, FilamentResult};

use crate::core::{self, Id};
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::path::Path;
use std::rc::Rc;

type ParseResult<T> = Result<T, Error<Rule>>;
// user data is the input program so that we can create ir::Id's
// that have a reference to the input string
type Node<'i> = pest_consume::Node<'i, Rule, Rc<str>>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("syntax.pest");

#[derive(Parser)]
#[grammar = "frontend/syntax.pest"]
pub struct FilamentParser;

impl FilamentParser {
    /// Parse a Calyx program into an AST representation.
    pub fn parse_file(path: &Path) -> FilamentResult<core::Namespace> {
        let content = &fs::read(path).map_err(|err| {
            errors::Error::InvalidFile(format!(
                "Failed to read {}: {}",
                path.to_string_lossy(),
                err.to_string()
            ))
        })?;
        let string_content = std::str::from_utf8(content)?;
        let inputs = FilamentParser::parse_with_userdata(
            Rule::file,
            string_content,
            Rc::from(string_content),
        )
        .map_err(|e| e.with_path(&path.to_string_lossy()))?;
        let input = inputs.single()?;
        Ok(FilamentParser::file(input)?)
    }
}

#[pest_consume::parser]
impl FilamentParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn semi(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<Id> {
        Ok(Id::from(input.as_str()))
    }

    fn bitwidth(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected valid bitwidth"))
    }

    // ================ Intervals =====================

    fn time_base(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(var)] => core::IntervalTime::Abstract(var),
            [bitwidth(time)] => core::IntervalTime::Concrete(time),
        ))
    }

    fn time(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_base(time)] => time,
        ))
    }

    fn exact(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn within(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn interval_type(input: Node) -> ParseResult<core::IntervalType> {
        Ok(match_nodes!(
            input.into_children();
            [within(_)] => core::IntervalType::Within,
            [exact(_)] => core::IntervalType::Exact,
        ))
    }

    fn interval(input: Node) -> ParseResult<core::Interval> {
        Ok(match_nodes!(
            input.into_children();
            [interval_type(tag), time(start), time(end)] => {
                core::Interval {
                    tag,
                    start,
                    end,
                }
            }
        ))
    }

    // ================ Signature =====================

    fn port_def(input: Node) -> ParseResult<core::Port> {
        Ok(match_nodes!(
            input.into_children();
            [interval(liveness), identifier(name), bitwidth(bitwidth)] => {
                core::Port {
                    liveness, name, bitwidth
                }
            }
        ))
    }

    fn abstract_var(input: Node) -> ParseResult<Vec<Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(vars)..] => vars.collect(),
        ))
    }

    fn inputs(input: Node) -> ParseResult<Vec<core::Port>> {
        Ok(match_nodes!(
            input.into_children();
            [port_def(ins)..] => ins.collect()
        ))
    }
    fn outputs(input: Node) -> ParseResult<Vec<core::Port>> {
        Ok(match_nodes!(
            input.into_children();
            [port_def(ins)..] => ins.collect()
        ))
    }

    fn io(input: Node) -> ParseResult<(Vec<core::Port>, Vec<core::Port>)> {
        Ok(match_nodes!(
            input.into_children();
            [] => (vec![], vec![]),
            [inputs(ins)] =>  (ins, vec![]),
            [outputs(outs)] =>  (vec![], outs),
            [inputs(ins), outputs(outs)] => (ins, outs),
        ))
    }

    fn component(input: Node) -> ParseResult<core::Component> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(name),
                abstract_var(abstract_vars),
                io(io),
            ] => {
                let (inputs, outputs) = io;
                core::Component {
                    name,
                    cells: vec![],
                    abstract_vars,
                    inputs,
                    outputs,
                    body: core::Body,
                }
            },
            [
                identifier(name),
                io(io),
            ] => {
                let (inputs, outputs) = io;
                core::Component {
                    name,
                    cells: vec![],
                    abstract_vars: vec![],
                    inputs,
                    outputs,
                    body: core::Body,
                }
            }
        ))
    }

    fn file(input: Node) -> ParseResult<core::Namespace> {
        Ok(match_nodes!(
            input.into_children();
            [component(components).., _EOI] => {
                core::Namespace {
                    components: components.collect()
                }
            }
        ))
    }
}
