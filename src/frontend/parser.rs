#![allow(clippy::upper_case_acronyms)]

//! Parser for Calyx programs.
use super::ast::InterfaceDef;
use super::{ast, IntervalTime};
use crate::errors::{self, FilamentResult, WithPos};
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::path::Path;
use std::rc::Rc;

/// Data associated with parsing the file.
#[derive(Clone)]
struct UserData {
    /// Input to the parser
    pub input: Rc<str>,
    /// Path of the file
    pub file: Rc<str>,
}

type ParseResult<T> = Result<T, Error<Rule>>;
// user data is the input program so that we can create ir::Id's
// that have a reference to the input string
type Node<'i> = pest_consume::Node<'i, Rule, UserData>;

type Ports = Vec<ast::PortDef>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("syntax.pest");

pub enum ExtOrComp {
    Ext((String, Vec<ast::Signature>)),
    Comp(ast::Component),
}

pub enum PdOrInt {
    Pd(ast::PortDef),
    Int((ast::Id, ast::Id, u64)),
    Un((ast::Id, u64)),
}

#[derive(Parser)]
#[grammar = "frontend/syntax.pest"]
pub struct FilamentParser;

impl FilamentParser {
    /// Parse a Calyx program into an AST representation.
    pub fn parse_file(path: &Path) -> FilamentResult<ast::Namespace> {
        let content = &fs::read(path).map_err(|err| {
            errors::Error::invalid_file(format!(
                "Failed to read {}: {}",
                path.to_string_lossy(),
                err
            ))
        })?;
        let string_content = std::str::from_utf8(content)?;

        let user_data = UserData {
            input: Rc::from(string_content),
            file: Rc::from(path.to_string_lossy()),
        };
        let inputs = FilamentParser::parse_with_userdata(
            Rule::file,
            string_content,
            user_data,
        )
        .map_err(|e| e.with_path(&path.to_string_lossy()))?;
        let input = inputs.single()?;
        Ok(FilamentParser::file(input)?)
    }

    fn get_span(node: &Node) -> errors::Span {
        let ud = node.user_data();
        errors::Span::new(
            node.as_span(),
            Rc::clone(&ud.file),
            Rc::clone(&ud.input),
        )
    }
}

#[pest_consume::parser]
impl FilamentParser {
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<ast::Id> {
        Ok(ast::Id::from(input.as_str()))
    }

    fn char(input: Node) -> ParseResult<&str> {
        Ok(input.as_str())
    }

    fn string_lit(input: Node) -> ParseResult<String> {
        Ok(match_nodes!(
            input.into_children();
            [char(c)..] => c.collect::<Vec<_>>().join("")
        ))
    }

    fn bitwidth(input: Node) -> ParseResult<u64> {
        input
            .as_str()
            .parse::<u64>()
            .map_err(|_| input.error("Expected valid bitwidth"))
    }

    // ================ Intervals =====================
    fn time_base(input: Node) -> ParseResult<IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(var)] => IntervalTime::Abstract(var),
            [time(l), time(r)] => IntervalTime::binop_max(l, r),
            [bitwidth(time)] => IntervalTime::Concrete(time),
        ))
    }
    fn time_expr(input: Node) -> ParseResult<IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_base(l), time(r)] => IntervalTime::binop_add(l, r),
        ))
    }

    fn time(input: Node) -> ParseResult<IntervalTime> {
        Ok(match_nodes!(
            input.into_children();
            [time_expr(time)] => time,
            [time_base(time)] => time
        ))
    }

    fn interval_range(input: Node) -> ParseResult<ast::Range> {
        Ok(match_nodes!(
            input.into_children();
            [time(start), time(end)] => ast::Range { start, end }
        ))
    }

    // ================ Signature =====================

    fn interface(input: Node) -> ParseResult<(ast::Id, u64)> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(tvar), bitwidth(len)] => (tvar, len),
        ))
    }

    fn port_def(input: Node) -> ParseResult<PdOrInt> {
        Ok(match_nodes!(
            input.clone().into_children();
            [interface((time_var, len)), identifier(name), bitwidth(_)] => {
                PdOrInt::Int((name, time_var, len))
            },
            [identifier(name), bitwidth(bitwidth)] => {
                PdOrInt::Un((name, bitwidth))
            },
            [interval_range(range), identifier(name), bitwidth(bitwidth)] => {
                PdOrInt::Pd(ast::PortDef::new(name, range.into(), bitwidth))
            },
            [interval_range(range), interval_range(exact), identifier(name), bitwidth(bitwidth)] => {
                PdOrInt::Pd(ast::PortDef::new(name, ast::Interval::from(range).with_exact(exact), bitwidth))
            }
        ))
    }

    fn abstract_var(input: Node) -> ParseResult<Vec<ast::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(vars)..] => vars.collect(),
        ))
    }

    fn ports(
        input: Node,
    ) -> ParseResult<(Ports, Vec<ast::InterfaceDef>, Vec<(ast::Id, u64)>)> {
        Ok(match_nodes!(
            input.into_children();
            [port_def(ins)..] => {
                let mut interface_signals = vec![];
                let mut ports = vec![];
                let mut unannotated_ports = vec![];
                for m in ins {
                    match m {
                        PdOrInt::Pd(port) => ports.push(port),
                        PdOrInt::Int(int) => interface_signals.push(int),
                        PdOrInt::Un(un) => unannotated_ports.push(un)
                    }
                }
                let interface_ports = interface_signals.into_iter().map(|(name, event, delay)| {
                    ast::InterfaceDef::new(name, event, delay)
                }).collect();
                (ports, interface_ports, unannotated_ports)
            }
        ))
    }

    fn arrow(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn io(
        input: Node,
    ) -> ParseResult<(Ports, Ports, Vec<InterfaceDef>, Vec<(ast::Id, u64)>)>
    {
        match_nodes!(
            input.clone().into_children();
            [arrow(_)] => Ok((vec![], vec![], vec![], vec![])),
            [ports((ins, interface, un)), arrow(_)] =>  Ok((ins, vec![], interface, un)),
            [arrow(_), ports((outs, out_interface, o_un))] =>  {
                if !out_interface.is_empty() {
                    Err(input.error("Output interface ports not supported"))
                } else if !o_un.is_empty() {
                    Err(input.error("Output ports cannot be unannotated"))
                } else {
                    Ok((vec![], outs, vec![], vec![]))
                }
            },
            [ports((ins, interface, un)), arrow(_), ports((outs, out_interface, o_un))] => {
                if !out_interface.is_empty() {
                    Err(input.error("Output interface ports not supported"))
                } else if !o_un.is_empty() {
                    Err(input.error("Output ports cannot be unannotated"))
                } else {
                    Ok((ins, outs, interface, un))
                }
            }
        )
    }

    // ================ Cells =====================
    fn instance(input: Node) -> ParseResult<ast::Instance> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), identifier(component)] => ast::Instance {
                name, component
            }
        ))
    }

    // ================ Assignments =====================
    fn port(input: Node) -> ParseResult<ast::Port> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(constant)] => ast::Port::Constant(constant),
            [identifier(name)] => ast::Port::ThisPort(name),
            [identifier(comp), identifier(name)] => ast::Port::CompPort {
                comp, name
            }
        ))
    }

    fn port_with_span(input: Node) -> ParseResult<(ast::Port, errors::Span)> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [port(port)] => (port, span),
        ))
    }

    fn arguments(input: Node) -> ParseResult<Vec<(ast::Port, errors::Span)>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [port_with_span(ports)..] => ports.collect(),
        ))
    }

    fn time_args(input: Node) -> ParseResult<Vec<IntervalTime>> {
        Ok(match_nodes!(
            input.into_children();
            [time(args)..] => args.collect(),
        ))
    }

    fn invocation(input: Node) -> ParseResult<ast::Invoke> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(bind),
                identifier(comp),
                time_args(abstract_vars),
                arguments(ports)
            ] => ast::Invoke::new(bind, comp, abstract_vars, Some(ports)).set_span(Some(span)),
            [
                identifier(bind),
                identifier(comp),
                time_args(abstract_vars),
            ] => ast::Invoke::new(bind, comp, abstract_vars, None).set_span(Some(span))
        ))
    }
    fn gte(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn lte(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn gt(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn lt(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn eq(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn constraint(input: Node) -> ParseResult<ast::Constraint> {
        Ok(match_nodes!(
            input.clone().into_children();
            [
                time(l),
                eq(_),
                time(r)
            ] => ast::Constraint::new(l, r, ast::OrderOp::Eq),
            [
                time(l),
                gt(_),
                time(r)
            ] => ast::Constraint::new(l, r, ast::OrderOp::Gt),
            [
                time(l),
                lt(_),
                time(r)
            ] => ast::Constraint::new(r, l, ast::OrderOp::Gt),
            [
                time(l),
                lte(_),
                time(r)
            ] => ast::Constraint::new(r, l, ast::OrderOp::Gte),
            [
                time(l),
                gte(_),
                time(r)
            ] => ast::Constraint::new(l, r, ast::OrderOp::Gte),
        ))
    }
    fn constraints(input: Node) -> ParseResult<Vec<ast::Constraint>> {
        Ok(match_nodes!(
            input.into_children();
            [] => Vec::default(),
            [constraint(cons)..] => cons.collect()
        ))
    }

    // ================ Component =====================
    fn signature(input: Node) -> ParseResult<ast::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(name),
                abstract_var(abstract_vars),
                io(io),
                constraints(constraints)
            ] => {
                let (inputs, outputs, interface_signals, unannotated_ports) = io;
                ast::Signature {
                    name,
                    abstract_vars,
                    unannotated_ports,
                    interface_signals: interface_signals.into_iter().collect(),
                    inputs,
                    outputs,
                    constraints,
                }
            },
            [
                identifier(name),
                io(io),
                constraints(constraints)
            ] => {
                let (inputs, outputs, interface_signals, unannotated_ports) = io;
                ast::Signature {
                    name,
                    abstract_vars: vec![],
                    unannotated_ports,
                    interface_signals: interface_signals.into_iter().collect(),
                    inputs,
                    outputs,
                    constraints
                }
            }
        ))
    }

    fn guard(input: Node) -> ParseResult<ast::Guard> {
        Ok(match_nodes!(
            input.into_children();
            [port(p)] => ast::Guard::Port(p),
            [port(p), guard(g)] => {
                ast::Guard::Or(Box::new(ast::Guard::Port(p)), Box::new(g))
            }
        ))
    }

    fn connect(input: Node) -> ParseResult<ast::Connect> {
        let span = Some(Self::get_span(&input));
        Ok(match_nodes!(
            input.into_children();
            [port(dst), port(src)] => ast::Connect::new(dst, src, None).set_span(span),
            [port(dst), guard(guard), port(src)] => {
                ast::Connect::new(dst, src, Some(guard)).set_span(span)
            }
        ))
    }

    fn fsm(input: Node) -> ParseResult<ast::Fsm> {
        let span = Some(Self::get_span(&input));
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), bitwidth(states), port(trigger)] => ast::Fsm::new(name, states, trigger).set_span(span)
        ))
    }

    fn command(input: Node) -> ParseResult<ast::Command> {
        Ok(match_nodes!(
            input.into_children();
            [invocation(assign)] => ast::Command::Invoke(assign),
            [instance(cell)] => ast::Command::Instance(cell),
            [connect(con)] => ast::Command::Connect(con),
            [fsm(fsm)] => ast::Command::Fsm(fsm),
        ))
    }

    fn component(input: Node) -> ParseResult<ast::Component> {
        let span = Self::get_span(&input);
        match_nodes!(
            input.into_children();
            [
                signature(sig),
                command(body)..
            ] => {
                Ok(ast::Component::new(sig, body.collect()).unwrap())
            }
        )
    }

    fn external(input: Node) -> ParseResult<(String, Vec<ast::Signature>)> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(path), signature(sigs)..] => (path, sigs.collect()),
        ))
    }

    fn comp_or_ext(input: Node) -> ParseResult<ExtOrComp> {
        Ok(match_nodes!(
            input.into_children();
            [external(sig)] => ExtOrComp::Ext(sig),
            [component(comp)] => ExtOrComp::Comp(comp),
        ))
    }

    fn imports(input: Node) -> ParseResult<Vec<String>> {
        Ok(match_nodes!(
            input.into_children();
            [string_lit(path)..] => path.collect()
        ))
    }

    fn file(input: Node) -> ParseResult<ast::Namespace> {
        Ok(match_nodes!(
            input.into_children();
            [imports(imps), comp_or_ext(mixed).., _EOI] => {
                let mut namespace = ast::Namespace {
                    imports: imps,
                    externs: vec![],
                    components: vec![],
                };
                for m in mixed {
                    match m {
                        ExtOrComp::Ext(sig) => namespace.externs.push(sig),
                        ExtOrComp::Comp(comp) => namespace.components.push(comp),
                    }
                }
                namespace
            }
        ))
    }
}
