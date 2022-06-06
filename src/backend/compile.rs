use calyx::errors::CalyxResult;
use calyx::frontend;
use calyx::ir;
use calyx::ir::RRC;
use itertools::Itertools;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use super::Fsm;
use super::TimeRep;
use crate::errors::Error;
use crate::{core, errors::FilamentResult};

struct Context<'a> {
    /// Mapping from names to signatures for components and externals.
    sigs: &'a HashMap<core::Id, &'a core::Signature<TimeRep>>,

    /// Mapping from instances to cells
    instances: HashMap<core::Id, RRC<ir::Cell>>,

    /// Mapping from invocation name to instance
    invokes: HashMap<core::Id, RRC<ir::Cell>>,

    /// Builder for the current component
    builder: ir::Builder<'a>,

    /// Mapping from name to FSMs
    fsms: HashMap<core::Id, Fsm>,
}

impl<'a> Context<'a> {
    fn new(
        sigs: &'a HashMap<core::Id, &'a core::Signature<TimeRep>>,
        builder: ir::Builder<'a>,
    ) -> Self {
        Context {
            sigs,
            builder,
            instances: HashMap::default(),
            invokes: HashMap::default(),
            fsms: HashMap::default(),
        }
    }

    fn get_sig(
        &self,
        comp: &core::Id,
    ) -> Vec<(ir::Id, u64, ir::Direction, ir::Attributes)> {
        let sig = self.sigs[comp];
        sig.inputs
            .iter()
            .chain(sig.interface_signals.iter())
            .map(|pd| {
                (
                    ir::Id::from(pd.name.id.clone()),
                    pd.bitwidth,
                    ir::Direction::Input,
                    ir::Attributes::default(),
                )
            })
            .chain(sig.outputs.iter().map(|pd| {
                (
                    ir::Id::from(pd.name.id.clone()),
                    pd.bitwidth,
                    ir::Direction::Output,
                    ir::Attributes::default(),
                )
            }))
            .collect_vec()
    }

    fn add_invoke(&mut self, inv: core::Id, comp: core::Id) {
        let cell = &self
            .instances
            .get(&comp)
            .unwrap_or_else(|| panic!("Unknown instance: {}", comp));
        self.invokes.insert(inv, Rc::clone(cell));
    }
}

fn compile_port(
    port: core::Port,
    ctx: &mut Context,
) -> (RRC<ir::Port>, Option<ir::Guard>) {
    match port {
        core::Port::ThisPort(p) => {
            let this = ctx.builder.component.signature.borrow();
            (this.get(p), None)
        }
        core::Port::CompPort { comp, name } => {
            if let Some(fsm) = ctx.fsms.get(&comp) {
                let cr = ctx.builder.add_constant(1, 1);
                let c = cr.borrow();
                (
                    c.get("out"),
                    Some(
                        fsm.event(&name, &mut ctx.builder)
                            .expect("Undefined port on fsm"),
                    ),
                )
            } else {
                let cell = ctx.invokes[&comp].borrow();
                (cell.get(name), None)
            }
        }
        core::Port::Constant(c) => {
            let cr = ctx.builder.add_constant(c, 32);
            let c = cr.borrow();
            (c.get("out"), None)
        }
    }
}

fn compile_guard(guard: core::Guard, ctx: &mut Context) -> ir::Guard {
    match guard {
        core::Guard::Or(g1, g2) => {
            let c1 = compile_guard(*g1, ctx);
            let c2 = compile_guard(*g2, ctx);
            c1 | c2
        }
        core::Guard::Port(p) => match p {
            core::Port::ThisPort(p) => {
                let this = ctx.builder.component.signature.borrow();
                this.get(p).into()
            }
            core::Port::CompPort { comp, name } => {
                if let Some(fsm) = ctx.fsms.get(&comp) {
                    fsm.event(&name, &mut ctx.builder)
                        .expect("Undefined port on fsm")
                } else {
                    let cell = ctx.invokes[&comp].borrow();
                    cell.get(name).into()
                }
            }
            core::Port::Constant(_) => {
                unreachable!("Constants cannot be in guards")
            }
        },
    }
}

fn compile_command(cmd: core::Command<TimeRep>, ctx: &mut Context) {
    match cmd {
        core::Command::When(core::When { commands, .. }) => {
            commands
                .into_iter()
                .for_each(|cmd| compile_command(cmd, ctx));
        }
        core::Command::Fsm(fsm) => {
            let name = fsm.name.clone();
            let f = Fsm::new(fsm, &mut ctx.builder);
            ctx.fsms.insert(name, f);
        }
        core::Command::Invoke(core::Invoke {
            bind, comp, ports, ..
        }) => {
            assert!(
                ports.is_none(),
                "Cannot compile high-level invoke statements"
            );
            ctx.add_invoke(bind, comp);
        }
        core::Command::Instance(core::Instance { name, component }) => {
            let cell = ctx.builder.add_component(
                name.id.clone(),
                component.id.clone(),
                ctx.get_sig(&component),
            );
            ctx.instances.insert(name, cell);
        }
        core::Command::Connect(core::Connect {
            dst, src, guard, ..
        }) => {
            let (dst, g) = compile_port(dst, ctx);
            assert!(g.is_none(), "Destination has a guard");
            let (src, g) = compile_port(src, ctx);
            let guard = match (guard, g) {
                (None, None) => ir::Guard::True,
                (None, Some(g)) => g,
                (Some(g), None) => compile_guard(g, ctx),
                (Some(_), Some(_)) => {
                    panic!("Source implies guard and is guarded")
                }
            };
            let assign = ctx.builder.build_assignment(dst, src, guard);
            ctx.builder.component.continuous_assignments.push(assign);
        }
    }
}

fn compile_component(
    comp: core::Component<TimeRep>,
    sigs: &HashMap<core::Id, &core::Signature<TimeRep>>,
    lib: &ir::LibrarySignatures,
) -> FilamentResult<ir::Component> {
    let interface_ports = [
        ("go".to_string(), 1, ir::Direction::Input),
        ("clk".to_string(), 1, ir::Direction::Input),
        ("reset".to_string(), 1, ir::Direction::Input),
        ("done".to_string(), 1, ir::Direction::Output),
    ];

    let sig = comp.sig;
    let ports = sig
        .inputs
        .iter()
        .chain(sig.interface_signals.iter())
        .map(|pd| {
            (
                pd.name.id.clone(),
                pd.bitwidth,
                ir::Direction::Input,
                ir::Attributes::default(),
            )
        })
        .chain(sig.outputs.iter().map(|pd| {
            (
                pd.name.id.clone(),
                pd.bitwidth,
                ir::Direction::Output,
                ir::Attributes::default(),
            )
        }))
        .chain(interface_ports.into_iter().map(|(name, bw, dir)| {
            let mut attrs = ir::Attributes::default();
            attrs.insert(name.clone(), 1);
            (name, bw, dir, attrs)
        }))
        .collect();

    let mut component = ir::Component::new(&sig.name.id, ports);
    let builder = ir::Builder::new(&mut component, lib).not_generated();
    let mut ctx = Context::new(sigs, builder);
    for cmd in comp.body {
        compile_command(cmd, &mut ctx);
    }
    Ok(component)
}

fn get_library() -> CalyxResult<ir::Context> {
    let mut ws = frontend::Workspace::construct(
        &Some("../calyx/primitives/core.futil".into()),
        &PathBuf::from("../calyx"),
    )?;

    // define a fake main component
    let main = frontend::ast::ComponentDef::new("main", vec![]);
    ws.components.push(main);
    let mut ctx = ir::from_ast::ast_to_ir(ws)?;
    ctx.components.retain(|c| c.name != "main");
    Ok(ctx)
}

pub fn compile(ns: core::Namespace<TimeRep>) -> FilamentResult<()> {
    let mut calyx_ctx =
        get_library().map_err(|err| Error::misc(format!("{:?}", err)))?;

    let sigs = ns
        .externs
        .iter()
        .flat_map(|(_, comps)| comps.iter().map(|s| (s.name.clone(), s)))
        .collect::<HashMap<_, _>>();

    for comp in ns.components {
        calyx_ctx.components.push(compile_component(
            comp,
            &sigs,
            &calyx_ctx.lib,
        )?);
    }

    let mut out = &mut std::io::stdout();
    for (path, prims) in calyx_ctx.lib.externs() {
        ir::Printer::write_extern(
            (&path, &prims.into_iter().map(|(_, v)| v).collect_vec()),
            &mut out,
        )?;
    }
    for comp in &calyx_ctx.components {
        ir::Printer::write_component(comp, &mut out)?;
        println!();
    }

    Ok(())
}
