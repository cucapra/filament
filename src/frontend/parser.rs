#![allow(clippy::upper_case_acronyms)]

//! Parser for Filament programs.
use crate::core::{self, Loc, TimeSub};
use crate::errors::{self, FilamentResult};
use crate::utils::{FileIdx, GPosIdx, GlobalPositionTable};
use itertools::Itertools;
use pest::pratt_parser::{Assoc, Op, PrattParser};
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

type Ports = Vec<Loc<core::PortDef>>;

// include the grammar file so that Cargo knows to rebuild this file on grammar changes
const _GRAMMAR: &str = include_str!("syntax.pest");

// Define the precedence of binary operations. We use `lazy_static` so that
// this is only ever constructed once.
lazy_static::lazy_static! {
    static ref PRATT: PrattParser<Rule> =
    PrattParser::new()
        .op(Op::infix(Rule::op_add, Assoc::Left) | Op::infix(Rule::op_sub, Assoc::Left))
        .op(Op::infix(Rule::op_mul, Assoc::Left) | Op::infix(Rule::op_div, Assoc::Left) | Op::infix(Rule::op_mod, Assoc::Left));
}

pub enum ExtOrComp {
    Ext((String, Vec<core::Signature>)),
    Comp(core::Component),
}

#[derive(Clone)]
pub enum FCons {
    ExprC(core::OrderConstraint<core::Expr>),
    TimeC(core::OrderConstraint<core::Time>),
}

pub enum Port {
    Pd(Loc<core::PortDef>),
    Int(core::InterfaceDef),
    Un((core::Id, u64)),
}

#[derive(Parser)]
#[grammar = "frontend/syntax.pest"]
pub struct FilamentParser;

impl FilamentParser {
    pub fn parse_file(path: &Path) -> FilamentResult<core::Namespace> {
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
        let (_, content) = GlobalPositionTable::as_ref().get_file_data(file);
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

    #[allow(clippy::result_large_err)]
    fn expr_helper(
        ud: UserData,
        pairs: pest::iterators::Pairs<Rule>,
    ) -> ParseResult<core::Expr> {
        PRATT
            .map_primary(|primary| match primary.as_rule() {
                Rule::expr_base => Self::expr_base(Node::new_with_user_data(
                    primary,
                    ud.clone(),
                )),
                x => unreachable!("Unexpected rule `{:?}' for expr_helper", x),
            })
            .map_infix(|lhs, op, rhs| {
                Ok(match op.as_rule() {
                    Rule::op_add => core::Expr::op(core::Op::Add, lhs?, rhs?),
                    Rule::op_sub => core::Expr::op(core::Op::Sub, lhs?, rhs?),
                    Rule::op_mul => core::Expr::op(core::Op::Mul, lhs?, rhs?),
                    Rule::op_div => core::Expr::op(core::Op::Div, lhs?, rhs?),
                    Rule::op_mod => core::Expr::op(core::Op::Mod, lhs?, rhs?),
                    _ => unreachable!("Unknown binary operator"),
                })
            })
            .parse(pairs)
    }
}

#[pest_consume::parser]
impl FilamentParser {
    #[allow(unused)]
    // This is used by rust-analzyer doesn't think so
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn pound(_i: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<Loc<core::Id>> {
        let sp = Self::get_span(&input);
        let id = core::Id::from(input.as_str());
        Ok(Loc::new(id, sp))
    }

    fn param_var(input: Node) -> ParseResult<Loc<core::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [pound(_), identifier(id)] => id,
        ))
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
    fn time(input: Node) -> ParseResult<Loc<core::Time>> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [identifier(ev), expr(sts)] => Ok(Loc::new(core::Time::new(ev.take(), sts.take()), sp)),
            [expr(sts), identifier(ev)] => Ok(Loc::new(core::Time::new(ev.take(), sts.take()), sp)),
            [identifier(ev)] => Ok(Loc::new(core::Time::new(ev.take(), core::Expr::default()), sp)),
            [expr(_)] => {
                Err(input.error("time expressions must have the form `E+n' where `E' is an event and `n' is a concrete number or sum of parameters"))
            }
        )
    }

    fn interval_range(input: Node) -> ParseResult<Loc<core::Range>> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [time(start), time(end)] => Loc::new(core::Range::new(start.take(), end.take()), sp)
        ))
    }

    // ================ Signature =====================
    fn interface(input: Node) -> ParseResult<core::Id> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(tvar)] => tvar.take(),
        ))
    }

    fn pow2(input: Node) -> ParseResult<core::UnFn> {
        Ok(core::UnFn::Pow2)
    }
    fn log2(input: Node) -> ParseResult<core::UnFn> {
        Ok(core::UnFn::Log2)
    }
    fn unknown_fn(input: Node) -> ParseResult<core::UnFn> {
        Err(input.error("Unknown function"))
    }
    fn un_fn(input: Node) -> ParseResult<core::UnFn> {
        Ok(match_nodes!(
            input.into_children();
            [pow2(_)] => core::UnFn::Pow2,
            [log2(_)] => core::UnFn::Log2,
            [unknown_fn(_)] => unreachable!(),
        ))
    }

    fn expr_base(input: Node) -> ParseResult<core::Expr> {
        Ok(match_nodes!(
            input.into_children();
            [param_var(id)] => id.take().into(),
            [bitwidth(c)] => c.into(),
            [un_fn(f), expr(e)] => core::Expr::func(f, e.take()),
            [expr(e)] => e.take(),
        ))
    }

    fn expr(input: Node) -> ParseResult<Loc<core::Expr>> {
        let sp = Self::get_span(&input);
        let ud = input.user_data().clone();
        Self::expr_helper(ud, input.into_pair().into_inner())
            .map(|e| Loc::new(e, sp))
    }

    fn port_def(input: Node) -> ParseResult<Port> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [interface(time_var), identifier(name), expr(_)] => {
                Ok(Port::Int(core::InterfaceDef::new(name, time_var)))
            },
            [identifier(name), expr(bitwidth)] => {
                match (&bitwidth.take()).try_into() {
                    Ok(n) => Ok(Port::Un((name.take(), n))),
                    Err(n) => Err(input.error(format!("port width must be concrete: {}", n.kind))),
                }
            },
            [interval_range(range), identifier(name), expr(bitwidth)] => {
                Ok(Port::Pd(Loc::new(core::PortDef::port(name, range, bitwidth), sp)))
            },
            [identifier(name), expr(len), bundle_typ((idx, live, width))] => {
                Ok(Port::Pd(Loc::new(core::Bundle::new(name, core::BundleType::new(idx, len, live, width)).into(), sp)))
            }
        )
    }

    fn delay(input: Node) -> ParseResult<Loc<TimeSub>> {
        let sp = Self::get_span(&input);
        let out = match_nodes!(
            input.into_children();
            [expr(n)] => n.take().into(),
            [time(l), time(r)] => l.take() - r.take(),
        );
        Ok(Loc::new(out, sp))
    }

    fn event_bind(input: Node) -> ParseResult<Loc<core::EventBind>> {
        let sp = Self::get_span(&input);
        let out = match_nodes!(
            input.into_children();
            [identifier(event), delay(d), time(t)] => core::EventBind::new(event, d, Some(t.take())),
            [identifier(event), delay(d)] => core::EventBind::new(event, d, None),
        );
        Ok(Loc::new(out, sp))
    }

    fn abstract_var(input: Node) -> ParseResult<Vec<Loc<core::EventBind>>> {
        let evs = match_nodes!(
            input.clone().into_children();
            [event_bind(vars)..] => vars.collect_vec()
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
    ) -> ParseResult<(Ports, Vec<core::InterfaceDef>, Vec<(core::Id, u64)>)>
    {
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
    ) -> ParseResult<(
        Ports,
        Ports,
        Vec<core::InterfaceDef>,
        Vec<(core::Id, u64)>,
    )> {
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
    fn conc_params(input: Node) -> ParseResult<Vec<Loc<core::Expr>>> {
        Ok(match_nodes!(
            input.into_children();
            [expr(vars)..] => vars.collect(),
        ))
    }
    fn instance(input: Node) -> ParseResult<Vec<core::Command>> {
        Ok(match_nodes!(
            input.clone().into_children();
            [identifier(name), identifier(component), conc_params(params)] => vec![
                core::Instance::new(name, component, params).into()
            ],
            [identifier(name), identifier(component), conc_params(params), invoke_args((abstract_vars, ports))] => {
                // Upper case the first letter of name
                let mut iname = name.as_ref().to_string();
                iname.make_ascii_uppercase();
                let iname = Loc::new(core::Id::from(iname), name.pos());
                if iname == name {
                    input.error("Generated Instance name conflicts with original name");
                }
                let instance = core::Instance::new(iname.clone(), component, params).into();
                let invoke = core::Invoke::new(name, iname, abstract_vars, Some(ports)).into();
                vec![instance, invoke]
            }
        ))
    }

    // ================ Assignments =====================

    fn dots(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn access(input: Node) -> ParseResult<Loc<core::Access>> {
        let sp = Self::get_span(&input);
        let n = match_nodes!(
            input.clone().into_children();
            [expr(l), dots(_), expr(r)] => core::Access::range(l.take(), r.take()),
            [expr(e)] => e.take().into()
        );
        Ok(Loc::new(n, sp))
    }

    fn port(input: Node) -> ParseResult<Loc<core::Port>> {
        let sp = Self::get_span(&input);
        let n = match_nodes!(
            input.into_children();
            [bitwidth(constant)] => core::Port::constant(constant),
            [identifier(name)] => core::Port::this(name),
            [identifier(name), access(range)] => core::Port::bundle(name, range),
            [identifier(comp), identifier(name)] => core::Port::inv_port(comp, name),
            [identifier(invoke), identifier(port), access(access)] => core::Port::inv_bundle(invoke, port, access),
            [identifier(name), expr(idx)] => core::Port::bundle(name, idx.map(|x| x.into())),
        );
        Ok(Loc::new(n, sp))
    }

    fn arguments(input: Node) -> ParseResult<Vec<Loc<core::Port>>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [port(ports)..] => ports.collect(),
        ))
    }

    fn time_args(input: Node) -> ParseResult<Vec<Loc<core::Time>>> {
        Ok(match_nodes!(
            input.into_children();
            [time(args)..] => args.collect(),
        ))
    }

    #[allow(clippy::type_complexity)]
    fn invoke_args(
        input: Node,
    ) -> ParseResult<(Vec<Loc<core::Time>>, Vec<Loc<core::Port>>)> {
        Ok(match_nodes!(
            input.into_children();
            [time_args(time_args), arguments(args)] => (time_args, args),
        ))
    }

    fn invocation(input: Node) -> ParseResult<core::Invoke> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(bind),
                identifier(comp),
                invoke_args((abstract_vars, ports))
            ] => core::Invoke::new(bind, comp, abstract_vars, Some(ports)),
            [
                identifier(bind),
                identifier(comp),
                time_args(abstract_vars),
            ] => core::Invoke::new(bind, comp, abstract_vars, None),
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

    /// Returns the order operation and whether it is reversed
    fn order_op(input: Node) -> ParseResult<(core::OrderOp, bool)> {
        match_nodes!(
            input.into_children();
            [gt(_)] => Ok((core::OrderOp::Gt, false)),
            [lt(_)] => Ok((core::OrderOp::Gt, true)),
            [gte(_)] => Ok((core::OrderOp::Gte, false)),
            [lte(_)] => Ok((core::OrderOp::Gte, true)),
            [eq(_)] => Ok((core::OrderOp::Eq, false)),
        )
    }

    fn constraint(input: Node) -> ParseResult<Loc<FCons>> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [
                time(l),
                order_op((op, rev)),
                time(r)
            ] => {
                let l = l.take();
                let r = r.take();
                let con = if !rev {
                    core::OrderConstraint::new(l, r, op)
                } else {
                    core::OrderConstraint::new(r, l, op)
                };
                Ok(Loc::new(FCons::TimeC(con), sp))
            },
            [
                expr(l),
                order_op((op, rev)),
                expr(r)
            ] => {
                let con = if !rev {
                    core::OrderConstraint::new(l.take(), r.take(), op)
                } else {
                    core::OrderConstraint::new(r.take(), l.take(), op)
                };
                Ok(Loc::new(FCons::ExprC(con), sp))
            }
        )
    }

    #[allow(clippy::type_complexity)]
    fn constraints(
        input: Node,
    ) -> ParseResult<(
        Vec<Loc<core::OrderConstraint<core::Expr>>>,
        Vec<Loc<core::OrderConstraint<core::Time>>>,
    )> {
        Ok(match_nodes!(
            input.into_children();
            [] => (vec![], vec![]),
            [constraint(cons)..] => {
                let (mut expr, mut time) = (vec![], vec![]);
                for con in cons {
                    let pos = con.pos();
                    match con.take() {
                        FCons::ExprC(c) => expr.push(Loc::new(c, pos)),
                        FCons::TimeC(c) => time.push(Loc::new(c, pos)),
                    }
                }
                (expr, time)
            }
        ))
    }

    // ================ Component =====================
    fn params(input: Node) -> ParseResult<Vec<Loc<core::Id>>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [param_var(params)..] => params.collect_vec(),
        ))
    }
    fn signature(input: Node) -> ParseResult<core::Signature> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(name),
                params(params),
                abstract_var(abstract_vars),
                io(io),
                constraints((expr_c, time_c))
            ] => {
                let (inputs, outputs, interface_signals, unannotated_ports) = io;
                core::Signature::new(
                    name,
                    params,
                    abstract_vars,
                    unannotated_ports,
                    interface_signals,
                    inputs,
                    outputs,
                    expr_c,
                    time_c,
                 )
            },
            [
                identifier(name),
                params(params),
                io(io),
                constraints((expr_c, time_c))
            ] => {
                let (inputs, outputs, interface_signals, unannotated_ports) = io;
                core::Signature::new(
                    name,
                    params,
                    vec![],
                    unannotated_ports,
                    interface_signals.into_iter().collect(),
                    inputs,
                    outputs,
                    expr_c,
                    time_c
                 )
            }
        ))
    }

    fn guard(input: Node) -> ParseResult<core::Guard> {
        Ok(match_nodes!(
            input.into_children();
            [port(p)] => p.take().into(),
            [port(p), guard(g)] => {
                core::Guard::or(p.take().into(), g)
            }
        ))
    }

    fn connect(input: Node) -> ParseResult<core::Connect> {
        Ok(match_nodes!(
            input.into_children();
            [port(dst), port(src)] => core::Connect::new(dst, src, None),
            [port(dst), guard(guard), port(src)] => {
                core::Connect::new(dst, src, Some(guard))
            }
        ))
    }

    fn fsm(input: Node) -> ParseResult<core::Fsm> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), bitwidth(states), port(trigger)] => core::Fsm::new(name.take(), states, trigger.take()),
        ))
    }

    fn expr_cmp(input: Node) -> ParseResult<core::OrderConstraint<core::Expr>> {
        Ok(match_nodes!(
            input.into_children();
            [expr(l), order_op((op, rev)), expr(r)] => {
                if !rev {
                    core::OrderConstraint::new(l.take(), r.take(), op)
                } else {
                    core::OrderConstraint::new(r.take(), l.take(), op)
                }
            }
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<core::If> {
        Ok(match_nodes!(
            input.into_children();
            [expr_cmp(cond), commands(then), commands(else_)] => {
                core::If::new(cond, then, else_)
            }
        ))
    }

    fn for_loop(input: Node) -> ParseResult<core::ForLoop> {
        Ok(match_nodes!(
            input.into_children();
            [param_var(var), expr(start), expr(end), commands(body)] => {
                core::ForLoop::new(var, start.take(), end.take(), body)
            }
        ))
    }

    fn bundle_typ(
        input: Node,
    ) -> ParseResult<(Loc<core::Id>, Loc<core::Range>, Loc<core::Expr>)> {
        Ok(match_nodes!(
            input.into_children();
            [param_var(param), interval_range(range), expr(width)] => (param, range, width),
        ))
    }

    fn bundle(input: Node) -> ParseResult<core::Bundle> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), expr(size), bundle_typ((param, range, width))] => core::Bundle::new(name, core::BundleType::new(param, size, range, width)),
        ))
    }

    fn implication(input: Node) -> ParseResult<core::Implication<core::Expr>> {
        Ok(match_nodes!(
            input.into_children();
            [expr_cmp(guard), expr_cmp(e)] => core::Implication::implies(guard, e),
            [expr_cmp(e)] => core::Implication::fact(e)
        ))
    }

    fn assume_w(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn assert_w(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn fact(input: Node) -> ParseResult<core::Fact> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [assume_w(_), implication(e)] => core::Fact::assume(Loc::new(e, sp)),
            [assert_w(_), implication(e)] => core::Fact::assert(Loc::new(e, sp)),
        ))
    }

    fn command(input: Node) -> ParseResult<Vec<core::Command>> {
        Ok(match_nodes!(
            input.into_children();
            [invocation(assign)] => vec![core::Command::Invoke(assign)],
            [instance(cmd)] => cmd,
            [connect(con)] => vec![core::Command::Connect(con)],
            [fsm(fsm)] => vec![core::Command::Fsm(fsm)],
            [for_loop(l)] => vec![core::Command::ForLoop(l)],
            [bundle(bl)] => vec![bl.into()],
            [if_stmt(if_)] => vec![if_.into()],
            [fact(a)] => vec![a.into()]
        ))
    }

    fn commands(input: Node) -> ParseResult<Vec<core::Command>> {
        Ok(match_nodes!(
            input.into_children();
            [command(cmd)..] => cmd.into_iter().flatten().collect(),
        ))
    }

    fn component(input: Node) -> ParseResult<core::Component> {
        match_nodes!(
            input.into_children();
            [
                signature(sig),
                command(body)..
            ] => {
                Ok(core::Component::new(sig, body.into_iter().flatten().collect()))
            }
        )
    }

    fn external(input: Node) -> ParseResult<(String, Vec<core::Signature>)> {
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

    fn file(input: Node) -> ParseResult<core::Namespace> {
        Ok(match_nodes!(
            input.into_children();
            [imports(imps), comp_or_ext(mixed).., _EOI] => {
                let mut namespace = core::Namespace {
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
