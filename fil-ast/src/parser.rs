#![allow(clippy::upper_case_acronyms)]

//! Parser for Filament programs.
use crate::{self as ast, Loc, TimeSub};
use fil_utils::{self as utils, FilamentResult};
use fil_utils::{FileIdx, GPosIdx, GlobalPositionTable};
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

type Ports = Vec<Loc<ast::PortDef>>;

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
    Ext((String, Vec<ast::Signature>)),
    Comp(ast::Component),
}

#[derive(Clone)]
pub enum FCons {
    ExprC(ast::OrderConstraint<ast::Expr>),
    TimeC(ast::OrderConstraint<ast::Time>),
}

pub enum Port {
    Pd(Loc<ast::PortDef>),
    Int(ast::InterfaceDef),
    Un((ast::Id, u64)),
}

#[derive(Parser)]
#[grammar = "syntax.pest"]
pub struct FilamentParser;

impl FilamentParser {
    pub fn parse_file(path: &Path) -> FilamentResult<ast::Namespace> {
        let time = std::time::Instant::now();
        let content = &fs::read(path).map_err(|err| {
            utils::Error::invalid_file(format!(
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
                .map_err(|e| {
                    utils::Error::misc(format!(
                        "Failed to parse {}: {}",
                        e.with_path(&path.to_string_lossy()),
                        path.to_string_lossy(),
                    ))
                })?;
        let input = inputs.single().map_err(|e| {
            utils::Error::misc(format!(
                "Failed to parse {}: {}",
                e.with_path(&path.to_string_lossy()),
                path.to_string_lossy(),
            ))
        })?;
        let out = FilamentParser::file(input).map_err(|e| {
            utils::Error::misc(format!(
                "Failed to parse {}",
                e.with_path(&path.to_string_lossy()),
            ))
        })?;
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
    ) -> ParseResult<ast::Expr> {
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
                    Rule::op_add => ast::Expr::op(ast::Op::Add, lhs?, rhs?),
                    Rule::op_sub => ast::Expr::op(ast::Op::Sub, lhs?, rhs?),
                    Rule::op_mul => ast::Expr::op(ast::Op::Mul, lhs?, rhs?),
                    Rule::op_div => ast::Expr::op(ast::Op::Div, lhs?, rhs?),
                    Rule::op_mod => ast::Expr::op(ast::Op::Mod, lhs?, rhs?),
                    _ => unreachable!("Unknown binary operator"),
                })
            })
            .parse(pairs)
    }
}

#[pest_consume::parser]
impl FilamentParser {
    #[allow(unused, non_snake_case)]
    // This is used by rust-analzyer doesn't think so
    fn EOI(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn quote(input: Node) -> ParseResult<()> {
        Ok(())
    }

    // ================ Literals =====================
    fn identifier(input: Node) -> ParseResult<Loc<ast::Id>> {
        let sp = Self::get_span(&input);
        let id = ast::Id::from(input.as_str());
        Ok(Loc::new(id, sp))
    }

    fn param_var(input: Node) -> ParseResult<Loc<ast::Id>> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(id)] => id,
        ))
    }

    fn event(input: Node) -> ParseResult<Loc<ast::Id>> {
        match_nodes!(
            input.clone().into_children();
            [quote(_), identifier(id)] => Ok(id),
            [identifier(id)] => Err(input.error(format!("try replacing this with '{id}. Event must start with a single quote")))
        )
    }

    fn some(_input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn opaque(_input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn sig_bind(input: Node) -> ParseResult<Loc<ast::SigBind>> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [param_var(param), expr(e)] => Ok(Loc::new(ast::SigBind::let_(param, e.take()), sp)),
            [opaque(_), param_var(param), constraints(cons)] => {
                let (expr, ev) = cons;
                if !ev.is_empty() {
                    Err(input.error("Cannot specify event constraints in an existential binding"))
                } else {
                    Ok(Loc::new(ast::SigBind::exists(param, true, expr), sp))
                }
            },
            [some(_), param_var(param), constraints(cons)] => {
                let (expr, ev) = cons;
                if !ev.is_empty() {
                    Err(input.error("Cannot specify event constraints in an existential binding"))
                } else {
                    Ok(Loc::new(ast::SigBind::exists(param, false, expr), sp))
                }
            }
        )
    }

    fn param_bind(input: Node) -> ParseResult<Loc<ast::ParamBind>> {
        let sp = Self::get_span(&input);
        let out = match_nodes!(
            input.into_children();
            [param_var(param), expr(e)] => ast::ParamBind::new(param, Some(e.take())),
            [param_var(param)] => ast::ParamBind::new(param, None)
        );
        Ok(Loc::new(out, sp))
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
    fn time(input: Node) -> ParseResult<Loc<ast::Time>> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [event(ev), expr(sts)] => Ok(Loc::new(ast::Time::new(ev.take(), sts.take()), sp)),
            [expr(sts), event(ev)] => Ok(Loc::new(ast::Time::new(ev.take(), sts.take()), sp)),
            [event(ev)] => Ok(Loc::new(ast::Time::new(ev.take(), ast::Expr::default()), sp)),
            [expr(_)] => {
                Err(input.error("time expressions must have the form `E+n' where `E' is an event and `n' is a concrete number or sum of parameters"))
            }
        )
    }

    fn interval_range(input: Node) -> ParseResult<Loc<ast::Range>> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [time(start), time(end)] => Loc::new(ast::Range::new(start.take(), end.take()), sp)
        ))
    }

    // ================ Signature =====================
    fn interface(input: Node) -> ParseResult<ast::Id> {
        Ok(match_nodes!(
            input.into_children();
            [event(tvar)] => tvar.take(),
        ))
    }

    fn builtin_fn(input: Node) -> ParseResult<ast::Fn> {
        Ok(match input.as_str() {
            "pow2" => ast::Fn::Pow2,
            "log2" => ast::Fn::Log2,
            "sin_bits" => ast::Fn::SinB,
            "cos_bits" => ast::Fn::CosB,
            "bit_rev" => ast::Fn::BitRev,
            _ => unreachable!(),
        })
    }
    fn unknown_fn(input: Node) -> ParseResult<ast::Fn> {
        Err(input.error("Unknown function"))
    }
    fn r#fn(input: Node) -> ParseResult<ast::Fn> {
        Ok(match_nodes!(
            input.into_children();
            [builtin_fn(f)] => f,
            [unknown_fn(_)] => unreachable!(),
        ))
    }

    fn expr_base(input: Node) -> ParseResult<ast::Expr> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(inst), identifier(param)] => ast::Expr::ParamAccess{ inst, param },
            [param_var(id)] => ast::Expr::abs(id),
            [bitwidth(c)] => c.into(),
            [r#fn(f), expr(exprs)..] => ast::Expr::func(f, exprs.into_iter().map(|e| e.take()).collect()),
            [expr(e)] => e.take(),
        ))
    }

    fn expr(input: Node) -> ParseResult<Loc<ast::Expr>> {
        let sp = Self::get_span(&input);
        let ud = input.user_data().clone();
        Self::expr_helper(ud, input.into_pair().into_inner())
            .map(|e| Loc::new(e, sp))
    }

    fn port_def(input: Node) -> ParseResult<Port> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [identifier(name), interface(time_var)] => {
                Ok(Port::Int(ast::InterfaceDef::new(name, time_var)))
            },
            [identifier(name), bitwidth(n)] => {
                Ok(Port::Un((name.take(), n)))
            },
            [bundle_def(bd)] => {
                Ok(Port::Pd(Loc::new(bd.into(), sp)))
            },
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

    fn event_bind(input: Node) -> ParseResult<Loc<ast::EventBind>> {
        let sp = Self::get_span(&input);
        let out = match_nodes!(
            input.into_children();
            [event(event), delay(d), time(t)] => ast::EventBind::new(event, d, Some(t.take())),
            [event(event), delay(d)] => ast::EventBind::new(event, d, None),
        );
        Ok(Loc::new(out, sp))
    }

    fn abstract_var(input: Node) -> ParseResult<Vec<Loc<ast::EventBind>>> {
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
    fn conc_params(input: Node) -> ParseResult<Vec<Loc<ast::Expr>>> {
        Ok(match_nodes!(
            input.into_children();
            [expr(vars)..] => vars.collect(),
        ))
    }
    fn instance(input: Node) -> ParseResult<Vec<ast::Command>> {
        Ok(match_nodes!(
            input.clone().into_children();
            [identifier(name), identifier(component), conc_params(params)] => vec![
                ast::Instance::new(name, component, params).into()
            ],
            [identifier(name), identifier(component), conc_params(params), invoke_args((abstract_vars, ports))] => {
                // Upper case the first letter of name
                let mut iname = name.as_ref().to_string();
                iname.make_ascii_uppercase();
                let iname = Loc::new(ast::Id::from(iname), name.pos());
                if iname == name {
                    input.error("Generated Instance name conflicts with original name");
                }
                let instance = ast::Instance::new(iname.clone(), component, params).into();
                let invoke = ast::Invoke::new(name, iname, abstract_vars, ports).into();
                vec![instance, invoke]
            }
        ))
    }

    // ================ Assignments =====================

    fn dots(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn access(input: Node) -> ParseResult<Loc<ast::Access>> {
        let sp = Self::get_span(&input);
        let n = match_nodes!(
            input.clone().into_children();
            [expr(l), dots(_), expr(r)] => ast::Access::range(l.take(), r.take()),
            [expr(e)] => e.take().into()
        );
        Ok(Loc::new(n, sp))
    }

    fn port(input: Node) -> ParseResult<Loc<ast::Port>> {
        let sp = Self::get_span(&input);
        match_nodes!(
            input.clone().into_children();
            [bitwidth(_)] => Err(input.error("constant ports are not supported. Use the `Const[Width, Val]' primitive instead.")),
            [identifier(name)] => Ok(Loc::new(ast::Port::this(name), sp)),
            [identifier(name), access(range)] => Ok(Loc::new(ast::Port::bundle(name, range), sp)),
            [identifier(comp), identifier(name)] => Ok(Loc::new(ast::Port::inv_port(comp, name), sp)),
            [identifier(invoke), identifier(port), access(access)] => Ok(Loc::new(ast::Port::inv_bundle(invoke, port, access), sp)),
            [identifier(name), expr(idx)] => Ok(Loc::new(ast::Port::bundle(name, idx.map(|x| x.into())), sp)),
        )
    }

    fn arguments(input: Node) -> ParseResult<Vec<Loc<ast::Port>>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [port(ports)..] => ports.collect(),
        ))
    }

    fn time_args(input: Node) -> ParseResult<Vec<Loc<ast::Time>>> {
        Ok(match_nodes!(
            input.into_children();
            [time(args)..] => args.collect(),
        ))
    }

    #[allow(clippy::type_complexity)]
    fn invoke_args(
        input: Node,
    ) -> ParseResult<(Vec<Loc<ast::Time>>, Vec<Loc<ast::Port>>)> {
        Ok(match_nodes!(
            input.into_children();
            [time_args(time_args), arguments(args)] => (time_args, args),
        ))
    }

    fn invocation(input: Node) -> ParseResult<ast::Invoke> {
        Ok(match_nodes!(
            input.into_children();
            [
                identifier(bind),
                identifier(comp),
                invoke_args((abstract_vars, ports))
            ] => ast::Invoke::new(bind, comp, abstract_vars, ports)
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
    fn order_op(input: Node) -> ParseResult<(ast::OrderOp, bool)> {
        match_nodes!(
            input.into_children();
            [gt(_)] => Ok((ast::OrderOp::Gt, false)),
            [lt(_)] => Ok((ast::OrderOp::Gt, true)),
            [gte(_)] => Ok((ast::OrderOp::Gte, false)),
            [lte(_)] => Ok((ast::OrderOp::Gte, true)),
            [eq(_)] => Ok((ast::OrderOp::Eq, false)),
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
                    ast::OrderConstraint::new(l, r, op)
                } else {
                    ast::OrderConstraint::new(r, l, op)
                };
                Ok(Loc::new(FCons::TimeC(con), sp))
            },
            [
                expr(l),
                order_op((op, rev)),
                expr(r)
            ] => {
                let con = if !rev {
                    ast::OrderConstraint::new(l.take(), r.take(), op)
                } else {
                    ast::OrderConstraint::new(r.take(), l.take(), op)
                };
                Ok(Loc::new(FCons::ExprC(con), sp))
            }
        )
    }

    #[allow(clippy::type_complexity)]
    fn constraints(
        input: Node,
    ) -> ParseResult<(
        Vec<Loc<ast::OrderConstraint<ast::Expr>>>,
        Vec<Loc<ast::OrderConstraint<ast::Time>>>,
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
    fn params(input: Node) -> ParseResult<Vec<Loc<ast::ParamBind>>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [param_bind(params)..] => params.collect_vec(),
        ))
    }
    fn sig_bindings(input: Node) -> ParseResult<Vec<Loc<ast::SigBind>>> {
        Ok(match_nodes!(
            input.into_children();
            [] => vec![],
            [sig_bind(params)..] => params.collect_vec(),
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
                sig_bindings(sig_binds),
                constraints((expr_c, time_c))
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
                    expr_c,
                    time_c,
                    sig_binds,
                 )
            },
            [
                identifier(name),
                params(params),
                io(io),
                sig_bindings(sig_binds),
                constraints((expr_c, time_c))
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
                    expr_c,
                    time_c,
                    sig_binds,
                 )
            }
        ))
    }

    fn connect(input: Node) -> ParseResult<ast::Connect> {
        Ok(match_nodes!(
            input.into_children();
            [port(dst), port(src)] => ast::Connect::new(dst, src),
        ))
    }

    fn expr_cmp(input: Node) -> ParseResult<ast::OrderConstraint<ast::Expr>> {
        Ok(match_nodes!(
            input.into_children();
            [expr(l), order_op((op, rev)), expr(r)] => {
                if !rev {
                    ast::OrderConstraint::new(l.take(), r.take(), op)
                } else {
                    ast::OrderConstraint::new(r.take(), l.take(), op)
                }
            }
        ))
    }

    fn if_stmt(input: Node) -> ParseResult<ast::If> {
        Ok(match_nodes!(
            input.into_children();
            [expr_cmp(cond), commands(then), commands(else_)] => ast::If::new(cond, then, else_),
            [expr_cmp(cond), commands(then)] => ast::If::new(cond, then, vec![])
        ))
    }

    fn for_loop(input: Node) -> ParseResult<ast::ForLoop> {
        Ok(match_nodes!(
            input.into_children();
            [param_var(var), expr(start), expr(end), commands(body)] => {
                ast::ForLoop::new(var, start.take(), end.take(), body)
            }
        ))
    }

    fn bundle_def(input: Node) -> ParseResult<ast::Bundle> {
        Ok(match_nodes!(
            input.into_children();
            [identifier(name), expr(size), bundle_typ((param, range, width))] => ast::Bundle::new(name, ast::BundleType::new(param, size, range, width)),
            // This is bundle with size 1, i.e., a port.
            [identifier(name), bundle_typ((param, range, width))] => ast::Bundle::new(
                name,
                ast::BundleType::new(param, ast::Expr::concrete(1).into(), range, width)
            ),
        ))
    }

    fn bundle_typ(
        input: Node,
    ) -> ParseResult<(Option<Loc<ast::Id>>, Loc<ast::Range>, Loc<ast::Expr>)>
    {
        Ok(match_nodes!(
            input.into_children();
            [param_var(param), interval_range(range), expr(width)] => (Some(param), range, width),
            [interval_range(range), expr(width)] => (None, range, width),
        ))
    }

    fn bundle(input: Node) -> ParseResult<ast::Bundle> {
        Ok(match_nodes!(
            input.into_children();
            [bundle_def(bd)] => bd
        ))
    }

    fn implication(input: Node) -> ParseResult<ast::Implication<ast::Expr>> {
        Ok(match_nodes!(
            input.into_children();
            [expr_cmp(guard), expr_cmp(e)] => ast::Implication::implies(guard, e),
            [expr_cmp(e)] => ast::Implication::fact(e)
        ))
    }

    fn assume_w(input: Node) -> ParseResult<()> {
        Ok(())
    }
    fn assert_w(input: Node) -> ParseResult<()> {
        Ok(())
    }

    fn fact(input: Node) -> ParseResult<ast::Fact> {
        let sp = Self::get_span(&input);
        Ok(match_nodes!(
            input.into_children();
            [assume_w(_), implication(e)] => ast::Fact::assume(Loc::new(e, sp)),
            [assert_w(_), implication(e)] => ast::Fact::assert(Loc::new(e, sp)),
        ))
    }

    fn param_let(input: Node) -> ParseResult<ast::ParamLet> {
        Ok(match_nodes!(
            input.into_children();
            [param_var(name), expr(expr)] => ast::ParamLet { name, expr: expr.take() }
        ))
    }

    fn exists(input: Node) -> ParseResult<ast::Exists> {
        Ok(match_nodes!(
            input.into_children();
            [param_var(param), expr(bind)] => ast::Exists { param, bind }
        ))
    }

    fn command(input: Node) -> ParseResult<Vec<ast::Command>> {
        Ok(match_nodes!(
            input.into_children();
            [invocation(assign)] => vec![ast::Command::Invoke(assign)],
            [instance(cmd)] => cmd,
            [connect(con)] => vec![ast::Command::Connect(con)],
            [for_loop(l)] => vec![ast::Command::ForLoop(l)],
            [bundle(bl)] => vec![bl.into()],
            [if_stmt(if_)] => vec![if_.into()],
            [param_let(l)] => vec![l.into()],
            [exists(e)] => vec![e.into()],
            [fact(a)] => vec![a.into()],
        ))
    }

    fn commands(input: Node) -> ParseResult<Vec<ast::Command>> {
        Ok(match_nodes!(
            input.into_children();
            [command(cmd)..] => cmd.into_iter().flatten().collect(),
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
                    toplevel: "main".to_string(),
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
