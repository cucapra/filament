#![allow(clippy::upper_case_acronyms)]

//! Parser for Calyx programs.
use crate::ast::param as ast;
use crate::core::{self, TimeSub};
use crate::errors::{self, FilamentResult, WithPos};
use crate::utils::{FileIdx, GPosIdx, GlobalPositionTable};
use itertools::Itertools;
use pest_consume::{match_nodes, Error, Parser};
use std::fs;
use std::path::Path;

/// Data associated with parsing the file.
#[derive(Clone)]
struct UserData {
    /// Index to the current file
    pub file: FileIdx,
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

pub enum Port {
    Pd(ast::PortDef),
    Int(ast::InterfaceDef),
    Un((ast::Id, u64)),
}

#[derive(Parser)]
#[grammar = "frontend/syntax.pest"]
pub struct FilamentParser;

impl FilamentParser {
    pub fn parse_file(path: &Path) -> FilamentResult<ast::Namespace> {
        let time = std::time::Instant::now();
        let content = &fs::read(path).map_err(|err| {
            errors::Error::invalid_file(format!(
                "Failed to read {}: {err}",
                path.to_string_lossy(),
            ))
        })?;
        // Add a new file to the position table
        let string_content = std::str::from_utf8(content)?.to_string();
        let file = GlobalPositionTable::as_mut()
            .add_file(path.to_string_lossy().to_string(), string_content);
        let user_data = UserData { file };
        let content = GlobalPositionTable::as_ref().get_source(file);
        // Parse the file
        let inputs =
            FilamentParser::parse_with_userdata(Rule::file, content, user_data)
                .map_err(|e| e.with_path(&path.to_string_lossy()))?;
        let input = inputs.single()?;
        let out = FilamentParser::file(input)?;
        log::info!(
            "Parsed `{}` in {}ms",
            path.to_string_lossy(),
            time.elapsed().as_millis()
        );
        Ok(out)
    }

    fn get_span(node: &Node) -> GPosIdx {
        let ud = node.user_data();
        let sp = node.as_span();
        let pos = GlobalPositionTable::as_mut().add_pos(
            ud.file,
            sp.start(),
            sp.end(),
        );
        GPosIdx(pos)
    }
}

#[pest_consume::parser]
impl FilamentParser {
    #[allow(unused)]
    // This is used by rust-analzyer doesn't think so
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<ast::Id> {
        let sp = Self::get_span(&input);
        let id = ast::Id::from(input.as_str());
        Ok(id.set_span(sp))
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
    fn time(input: Node) -> ParseResult<ast::TimeRep> {
        match_nodes!(
            input.clone().into_children();
            [identifier(ev), port_width(sts)..] => Ok(core::ConcTime::new(ev, sts.collect_vec())),
            [bitwidth(_)] => {
                Err(input.error("Time expressions must have the form `E+n' where `E' is an event and `n' is a concrete number"))
            }
        )
    }

    fn interval_range(input: Node) -> ParseResult<ast::Range> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [time(start), time(end)] => ast::Range::new(start, end).set_span(sp)
        ))
    }

    // ================ Signature =====================

    fn interface(input: Node) -> ParseResult<ast::Id> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(tvar)] => tvar,
        ))
    }

    fn port_width(input: Node) -> ParseResult<ast::PortParam> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id)] => ast::PortParam::Var(id),
            [bitwidth(c)] => ast::PortParam::Const(c),
        ))
    }

    fn port_def(input: Node) -> ParseResult<Port> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.clone().into_children();
            [interface(time_var), identifier(name), port_width(_)] => {
                Port::Int(ast::InterfaceDef::new(name, time_var).set_span(sp))
            },
            [identifier(name), port_width(bitwidth)] => {
                match bitwidth {
                    ast::PortParam::Const(c) => Port::Un((name, c)),
                    ast::PortParam::Var(_) => todo!(),
                }
            },
            [interval_range(range), identifier(name), port_width(bitwidth)] => {
                Port::Pd(ast::PortDef::new(name, range, bitwidth).set_span(sp))
            }
        ))
    }

    fn delay(input: Node) -> ParseResult<TimeSub> {
        Ok(match_nodes!(
            input.into_children();
            [bitwidth(n)] => TimeSub::unit(n),
            [time(l), time(r)] => TimeSub::sym(l, r),
        ))
    }

    fn event_bind(input: Node) -> ParseResult<ast::EventBind> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [identifier(event), delay(d), time(t)] => ast::EventBind::new(event, d, Some(t)).set_span(sp),
            [identifier(event), delay(d)] => ast::EventBind::new(event, d, None).set_span(sp),
        ))
    }

    fn abstract_var(input: Node) -> ParseResult<Vec<ast::EventBind>> {
        let evs: Vec<ast::EventBind> = match_nodes!(
            input.clone().into_children();
            [event_bind(vars)..] => vars.collect()
        );
        let mut opts_started = false;
        for ev in &evs {
            if ev.default.is_some() {
                if !opts_started {
                    opts_started = true;
                }
            } else if opts_started {
                return Err(input.error("Default values must be specified before non-default values"));
            }
        }
        Ok(evs)
    }

    #[allow(clippy::type_complexity)]
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
                        Port::Pd(port) => ports.push(port),
                        Port::Int(int) => interface_signals.push(int),
                        Port::Un(un) => unannotated_ports.push(un)
                    }
                }
                (ports, interface_signals, unannotated_ports)
            }
        ))
    }

    fn arrow(input: Node) -> ParseResult<()> {
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    fn io(
        input: Node,
    ) -> ParseResult<(Ports, Ports, Vec<ast::InterfaceDef>, Vec<(ast::Id, u64)>)>
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
    fn conc_params(input: Node) -> ParseResult<Vec<ast::PortParam>> {
        Ok(match_nodes!(
            input.into_children();
            [port_width(vars)..] => vars.collect(),
        ))
    }
    fn instance(input: Node) -> ParseResult<Vec<ast::Command>> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.clone().into_children();
            [identifier(name), identifier(component), conc_params(params)] => vec![
                ast::Instance::new(name, component, params).set_span(sp).into()
            ],
            [identifier(name), identifier(component), conc_params(params), invoke_args((abstract_vars, ports))] => {
                // Upper case the first letter of name
                let mut iname = name.id().to_string();
                iname.make_ascii_uppercase();
                let iname = ast::Id::from(iname).set_span(component.copy_span());
                if iname == name {
                    input.error("Generated Instance name conflicts with original name");
                }
                let instance = ast::Instance::new(iname.clone(), component, params).set_span(sp).into();
                let invoke = ast::Invoke::new(name, iname, abstract_vars, Some(ports)).set_span(sp).into();
                vec![instance, invoke]
            }
        ))
    }

    // ================ Assignments =====================
    fn port(input: Node) -> ParseResult<ast::Port> {
        let sp = Self::get_span(&input);
        let n = match_nodes!(
            input.into_children();
            [bitwidth(constant)] => ast::Port::constant(constant),
            [identifier(name)] => ast::Port::this(name),
            [identifier(comp), identifier(name)] => ast::Port::comp(comp, name),
        );
        Ok(n.set_span(sp))
    }

    fn arguments(input: Node) -> ParseResult<Vec<ast::Port>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [port(ports)..] => ports.collect(),
        ))
    }

    fn time_args(input: Node) -> ParseResult<Vec<ast::TimeRep>> {
        Ok(match_nodes!(
            input.into_children();
            [time(args)..] => args.collect(),
        ))
    }

    fn invoke_args(
        input: Node,
    ) -> ParseResult<(Vec<ast::TimeRep>, Vec<ast::Port>)> {
        Ok(match_nodes!(
            input.into_children();
            [time_args(time_args), arguments(args)] => (time_args, args),
        ))
    }

    fn invocation(input: Node) -> ParseResult<ast::Invoke> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(bind),
                identifier(comp),
                invoke_args((abstract_vars, ports))
            ] => ast::Invoke::new(bind, comp, abstract_vars, Some(ports)).set_span(span),
            [
                identifier(bind),
                identifier(comp),
                time_args(abstract_vars),
            ] => ast::Invoke::new(bind, comp, abstract_vars, None).set_span(span)
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
        let cons = match_nodes!(
            input.clone().into_children();
            [
                time(l),
                eq(_),
                time(r)
            ] => ast::CBT::eq(l, r),
            [
                time(l),
                gt(_),
                time(r)
            ] => ast::CBT::lt(r, l),
            [
                time(l),
                lt(_),
                time(r)
            ] => ast::CBT::lt(l, r),
            [
                time(l),
                lte(_),
                time(r)
            ] => ast::CBT::gte(r, l),
            [
                time(l),
                gte(_),
                time(r)
            ] => ast::CBT::gte(l, r),
        );
        Ok(ast::Constraint::base(cons))
    }
    fn constraints(input: Node) -> ParseResult<Vec<ast::Constraint>> {
        Ok(match_nodes!(
            input.into_children();
            [] => Vec::default(),
            [constraint(cons)..] => cons.collect()
        ))
    }

    // ================ Component =====================
    fn params(input: Node) -> ParseResult<Vec<ast::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [identifier(params)..] => params.collect(),
        ))
    }
    fn signature(input: Node) -> ParseResult<ast::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(name),
                params(params),
                abstract_var(abstract_vars),
                io(io),
                constraints(constraints)
            ] => {
                let (inputs, outputs, interface_signals, unannotated_ports) = io;
                ast::Signature::new(
                    name,
                    params,
                    abstract_vars,
                    unannotated_ports,
                    interface_signals,
                    inputs,
                    outputs,
                    constraints,
                 )
            },
            [
                identifier(name),
                params(params),
                io(io),
                constraints(constraints)
            ] => {
                let (inputs, outputs, interface_signals, unannotated_ports) = io;
                ast::Signature::new(
                    name,
                    params,
                    vec![],
                    unannotated_ports,
                    interface_signals.into_iter().collect(),
                    inputs,
                    outputs,
                    constraints
                 )
            }
        ))
    }

    fn guard(input: Node) -> ParseResult<ast::Guard> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [port(p)] => p.into(),
            [port(p), guard(g)] => {
                ast::Guard::or(p.into(), g).set_span(sp)
            }
        ))
    }

    fn connect(input: Node) -> ParseResult<ast::Connect> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [port(dst), port(src)] => ast::Connect::new(dst, src, None).set_span(span),
            [port(dst), guard(guard), port(src)] => {
                ast::Connect::new(dst, src, Some(guard)).set_span(span)
            }
        ))
    }

    fn fsm(input: Node) -> ParseResult<ast::Fsm> {
        let span = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), bitwidth(states), port(trigger)] => ast::Fsm::new(name, states, trigger).set_span(span)
        ))
    }

    fn command(input: Node) -> ParseResult<Vec<ast::Command>> {
        Ok(match_nodes!(
            input.into_children();
            [invocation(assign)] => vec![ast::Command::Invoke(assign)],
            [instance(cmd)] => cmd,
            [connect(con)] => vec![ast::Command::Connect(con)],
            [fsm(fsm)] => vec![ast::Command::Fsm(fsm)],
        ))
    }

    fn component(input: Node) -> ParseResult<ast::Component> {
        match_nodes!(
            input.into_children();
            [
                signature(sig),
                command(body)..
            ] => {
                Ok(ast::Component::new(sig, body.into_iter().flatten().collect()))
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
