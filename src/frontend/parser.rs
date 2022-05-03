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

pub enum ExtOrComp {
    Ext(core::Signature),
    Comp(core::Component),
}

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
                err
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
    fn plus(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn max(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn time_base(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(var)] => core::IntervalTime::Abstract(var),
            [bitwidth(time)] => core::IntervalTime::Concrete(time),
        ))
    }
    fn time_expr(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_base(l), plus(_), time(r)] => core::IntervalTime::BinOp {
                left: Box::new(l),
                right: Box::new(r),
                op: core::TimeOp::Add,
            },
            [max(_), time(l), time(r)] => core::IntervalTime::BinOp {
                left: Box::new(l),
                right: Box::new(r),
                op: core::TimeOp::Max,
            },
        ))
    }

    fn time(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_expr(time)] => time,
            [time_base(time)] => time
        ))
    }

    fn interval(input: Node) -> ParseResult<core::Interval> {
        Ok(match_nodes!(
            input.into_children();
            [time(start), time(end)] => {
                core::Interval {
                    start,
                    end,
                }
            }
        ))
    }

    // ================ Signature =====================

    fn port_def(input: Node) -> ParseResult<core::PortDef> {
        Ok(match_nodes!(
            input.into_children();
            [interval(liveness), identifier(name), bitwidth(bitwidth)] => {
                core::PortDef {
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

    fn ports(input: Node) -> ParseResult<Vec<core::PortDef>> {
        Ok(match_nodes!(
            input.into_children();
            [port_def(ins)..] => ins.collect()
        ))
    }

    fn arrow(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn io(
        input: Node,
    ) -> ParseResult<(Vec<core::PortDef>, Vec<core::PortDef>)> {
        Ok(match_nodes!(
            input.into_children();
            [arrow(_)] => (vec![], vec![]),
            [ports(ins), arrow(_)] =>  (ins, vec![]),
            [arrow(_), ports(outs)] =>  (vec![], outs),
            [ports(ins), arrow(_), ports(outs)] => (ins, outs),
        ))
    }

    // ================ Cells =====================
    fn instance(input: Node) -> ParseResult<core::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), identifier(component)] => core::Cell {
                name, component
            }
        ))
    }

    // ================ Assignments =====================
    fn port(input: Node) -> ParseResult<core::Port> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(constant)] => core::Port::Constant(constant),
            [identifier(name)] => core::Port::ThisPort(name),
            [identifier(comp), identifier(name)] => core::Port::CompPort {
                comp, name
            }
        ))
    }

    fn arguments(input: Node) -> ParseResult<Vec<core::Port>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [port(ports)..] => ports.collect(),
        ))
    }

    fn time_args(input: Node) -> ParseResult<Vec<core::IntervalTime>> {
        Ok(match_nodes!(
            input.into_children();
            [time(args)..] => args.collect(),
        ))
    }

    fn invocation_expr(input: Node) -> ParseResult<core::Invocation> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(comp),
                time_args(abstract_vars),
                arguments(ports)
            ] => core::Invocation {
                comp, abstract_vars, ports
            }
        ))
    }

    fn invocation(input: Node) -> ParseResult<core::Invoke> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(bind),
                invocation_expr(rhs)
            ] => core::Invoke {
                bind, rhs
            }
        ))
    }

    // ================ Component =====================
    fn signature(input: Node) -> ParseResult<core::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(name),
                abstract_var(abstract_vars),
                io(io),
            ] => {
                let (inputs, outputs) = io;
                core::Signature {
                    name,
                    abstract_vars,
                    inputs,
                    outputs,
                }
            },
            [
                identifier(name),
                io(io),
            ] => {
                let (inputs, outputs) = io;
                core::Signature {
                    name,
                    abstract_vars: vec![],
                    inputs,
                    outputs,
                }
            }
        ))
    }

    fn when(input: Node) -> ParseResult<core::When> {
        Ok(match_nodes!(
            input.into_children();
            [time(time), command(body)..] => core::When {
                time, commands: body.collect()
            }
        ))
    }

    fn connect(input: Node) -> ParseResult<core::Connect> {
        Ok(match_nodes!(
            input.into_children();
            [port(dst), port(src)] => core::Connect {
                src, dst
            }
        ))
    }

    fn command(input: Node) -> ParseResult<core::Command> {
        Ok(match_nodes!(
            input.into_children();
            [invocation(assign)] => core::Command::Invoke(assign),
            [instance(cell)] => core::Command::Instance(cell),
            [when(wh)] => core::Command::When(wh),
            [connect(con)] => core::Command::Connect(con),
        ))
    }

    fn component(input: Node) -> ParseResult<core::Component> {
        Ok(match_nodes!(
            input.into_children();
            [
                signature(sig),
                command(body)..
            ] => {
                core::Component {
                    sig,
                    body: body.collect(),
                }
            }
        ))
    }

    fn external(input: Node) -> ParseResult<core::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [signature(sig)] => sig,
        ))
    }

    fn comp_or_ext(input: Node) -> ParseResult<ExtOrComp> {
        Ok(match_nodes!(
            input.into_children();
            [external(sig)] => ExtOrComp::Ext(sig),
            [component(comp)] => ExtOrComp::Comp(comp),
        ))
    }

    fn file(input: Node) -> ParseResult<core::Namespace> {
        Ok(match_nodes!(
            input.into_children();
            [comp_or_ext(mixed).., _EOI] => {
                let mut namespace = core::Namespace {
                    signatures: vec![],
                    components: vec![],
                };
                for m in mixed {
                    match m {
                        ExtOrComp::Ext(sig) => namespace.signatures.push(sig),
                        ExtOrComp::Comp(comp) => namespace.components.push(comp),
                    }
                }
                namespace
            }
        ))
    }
}
