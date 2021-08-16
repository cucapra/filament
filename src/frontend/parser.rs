#![allow(clippy::upper_case_acronyms)]

//! Parser for Calyx programs.
use crate::errors::{self, FilamentResult};

use crate::core::{self, Id};
use pest::prec_climber::{Assoc, Operator, PrecClimber};
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

// Define the precedence of binary operations. We use `lazy_static` so that
// this is only ever constructed once.
lazy_static::lazy_static! {
    static ref PRECCLIMBER: PrecClimber<Rule> = PrecClimber::new(
        vec![
            // loosest binding
            Operator::new(Rule::binop_add, Assoc::Left) | Operator::new(Rule::binop_sub, Assoc::Left),
            // tighest binding
        ]
    );
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
    fn time_port(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(cell), identifier(name)] => core::IntervalTime::Port {
                cell, name
            }
        ))
    }

    fn time_base(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_port(port)] => port,
            [identifier(var)] => core::IntervalTime::Abstract(var),
            [bitwidth(time)] => core::IntervalTime::Concrete(time),
        ))
    }

    #[prec_climb(time_base, PRECCLIMBER)]
    fn time_expr(
        l: core::IntervalTime,
        op: Node,
        r: core::IntervalTime,
    ) -> ParseResult<core::IntervalTime> {
        let op = match op.as_rule() {
            Rule::binop_add => core::TimeOp::Add,
            Rule::binop_sub => core::TimeOp::Sub,
            _ => unreachable!(),
        };
        Ok(core::IntervalTime::BinOp {
            left: Box::new(l),
            right: Box::new(r),
            op,
        })
    }

    fn time(input: Node) -> ParseResult<core::IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_expr(time)] => time,
            [time_base(time)] => time
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

    fn io(input: Node) -> ParseResult<(Vec<core::PortDef>, Vec<core::PortDef>)> {
        Ok(match_nodes!(
            input.into_children();
            [arrow(_)] => (vec![], vec![]),
            [ports(ins), arrow(_)] =>  (ins, vec![]),
            [arrow(_), ports(outs)] =>  (vec![], outs),
            [ports(ins), arrow(_), ports(outs)] => (ins, outs),
        ))
    }

    // ================ Cells =====================
    fn cell_def(input: Node) -> ParseResult<core::Cell> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), identifier(component)] => core::Cell {
                name, component
            }
        ))
    }

    fn cells(input: Node) -> ParseResult<Vec<core::Cell>> {
        Ok(match_nodes!(
            input.into_children();
            [cell_def(cells)..] => cells.collect()
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

    fn invocation(input: Node) -> ParseResult<core::Invocation> {
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

    fn assignment(input: Node) -> ParseResult<core::Assignment> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(bind),
                invocation(rhs)
            ] => core::Assignment {
                bind, rhs
            }
        ))
    }

    // ================ Component =====================
    fn component(input: Node) -> ParseResult<core::Component> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(name),
                abstract_var(abstract_vars),
                io(io),
                cells(cells),
                assignment(assignments)..
            ] => {
                let (inputs, outputs) = io;
                let sig = core::Signature {
                    abstract_vars,
                    inputs,
                    outputs,
                };
                core::Component {
                    name,
                    sig,
                    cells,
                    assignments: assignments.collect(),
                }
            },
            [
                identifier(name),
                io(io),
                cells(cells),
                assignment(assignments)..
            ] => {
                let (inputs, outputs) = io;
                let sig = core::Signature {
                    abstract_vars: vec![],
                    inputs,
                    outputs,
                };
                core::Component {
                    name,
                    cells,
                    sig,
                    assignments: assignments.collect(),
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
