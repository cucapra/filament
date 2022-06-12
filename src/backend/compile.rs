use calyx::errors::CalyxResult;
use calyx::frontend;
use calyx::ir;
use calyx::ir::RRC;
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use super::Fsm;
use crate::cmdline::Opts;
use crate::errors::Error;
use crate::errors::FilamentResult;
use crate::event_checker::ast;

pub struct Context<'a> {
    /// Builder for the current component
    pub builder: ir::Builder<'a>,

    /// Mapping from names to signatures for components and externals.
    sigs: &'a HashMap<ast::Id, Vec<ir::PortDef<u64>>>,

    /// Mapping from instances to cells
    instances: HashMap<ast::Id, RRC<ir::Cell>>,

    /// Mapping from invocation name to instance
    invokes: HashMap<ast::Id, RRC<ir::Cell>>,

    /// Mapping from name to FSMs
    fsms: HashMap<ast::Id, Fsm>,
}

impl Context<'_> {
    pub fn compile_port(
        &mut self,
        port: &ast::Port,
    ) -> (RRC<ir::Port>, Option<ir::Guard>) {
        match &port.typ {
            ast::PortType::ThisPort(p) => {
                let this = self.builder.component.signature.borrow();
                (this.get(p), None)
            }
            ast::PortType::CompPort { comp, name } => {
                if let Some(fsm) = self.fsms.get(comp) {
                    let cr = self.builder.add_constant(1, 1);
                    let c = cr.borrow();
                    (
                        c.get("out"),
                        Some(
                            fsm.event(name, &mut self.builder)
                                .expect("Undefined port on fsm"),
                        ),
                    )
                } else {
                    let cell = self.invokes[comp].borrow();
                    (cell.get(name), None)
                }
            }
            ast::PortType::Constant(c) => {
                let cr = self.builder.add_constant(*c, 32);
                let c = cr.borrow();
                (c.get("out"), None)
            }
        }
    }
}

impl<'a> Context<'a> {
    fn new(
        sigs: &'a HashMap<ast::Id, Vec<ir::PortDef<u64>>>,
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

    fn get_sig(&self, comp: &ast::Id) -> &Vec<ir::PortDef<u64>> {
        &self.sigs[comp]
    }

    fn add_invoke(&mut self, inv: ast::Id, comp: ast::Id) {
        let cell = &self
            .instances
            .get(&comp)
            .unwrap_or_else(|| panic!("Unknown instance: {}", comp));
        self.invokes.insert(inv, Rc::clone(cell));
    }
}

fn compile_guard(guard: ast::Guard, ctx: &mut Context) -> ir::Guard {
    match guard {
        ast::Guard::Or(g1, g2) => {
            let c1 = compile_guard(*g1, ctx);
            let c2 = compile_guard(*g2, ctx);
            c1 | c2
        }
        ast::Guard::Port(p) => match p.typ {
            ast::PortType::ThisPort(p) => {
                let this = ctx.builder.component.signature.borrow();
                this.get(p).into()
            }
            ast::PortType::CompPort { comp, name } => {
                if let Some(fsm) = ctx.fsms.get(&comp) {
                    fsm.event(&name, &mut ctx.builder)
                        .expect("Undefined port on fsm")
                } else {
                    let cell = ctx.invokes[&comp].borrow();
                    cell.get(name).into()
                }
            }
            ast::PortType::Constant(_) => {
                unreachable!("Constants cannot be in guards")
            }
        },
    }
}

fn compile_command(cmd: ast::Command, ctx: &mut Context) {
    match cmd {
        ast::Command::Fsm(fsm) => {
            let name = fsm.name.clone();
            let f = Fsm::new(fsm, ctx);
            ctx.fsms.insert(name, f);
        }
        ast::Command::Invoke(ast::Invoke {
            bind, comp, ports, ..
        }) => {
            assert!(
                ports.is_none(),
                "Cannot compile high-level invoke statements"
            );
            ctx.add_invoke(bind, comp);
        }
        ast::Command::Instance(ast::Instance { name, component }) => {
            let cell = ctx.builder.add_component(
                name.id.clone(),
                component.id.clone(),
                ctx.get_sig(&component).clone(),
            );
            ctx.instances.insert(name, cell);
        }
        ast::Command::Connect(ast::Connect {
            dst, src, guard, ..
        }) => {
            let (dst, g) = ctx.compile_port(&dst);
            assert!(g.is_none(), "Destination has a guard");
            let (src, g) = ctx.compile_port(&src);
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

fn as_port_defs(sig: &ast::Signature, extend: bool) -> Vec<ir::PortDef<u64>> {
    let mut ports: Vec<ir::PortDef<u64>> = sig
        .inputs
        .iter()
        .chain(sig.interface_signals.iter())
        .map(|pd| {
            (
                ir::Id::from(pd.name.id.clone()),
                pd.bitwidth,
                ir::Direction::Input,
            )
                .into()
        })
        .chain(sig.outputs.iter().map(|pd| {
            (
                ir::Id::from(pd.name.id.clone()),
                pd.bitwidth,
                ir::Direction::Output,
            )
                .into()
        }))
        .collect_vec();

    // Add annotations for interface ports
    let mut interface_ports = INTERFACE_PORTS
        .iter()
        .map(|pd| pd.0)
        .collect::<HashSet<_>>();
    for pd in &mut ports {
        if interface_ports.contains(pd.name.as_ref()) {
            interface_ports.remove(pd.name.as_ref());
            pd.attributes.insert(pd.name.clone(), 1);
        }
    }
    // Add missing interface ports
    if extend {
        for name in interface_ports {
            let mut attrs = ir::Attributes::default();
            attrs.insert(name, 1);
            ports.push(ir::PortDef {
                name: name.into(),
                width: 1,
                direction: ir::Direction::Input,
                attributes: attrs,
            })
        }
    }

    ports
}

const INTERFACE_PORTS: [(&str, u64, calyx::ir::Direction); 2] = [
    ("clk", 1, ir::Direction::Input),
    ("reset", 1, ir::Direction::Input),
];

fn compile_component(
    comp: ast::Component,
    sigs: &HashMap<ast::Id, Vec<ir::PortDef<u64>>>,
    lib: &ir::LibrarySignatures,
) -> FilamentResult<ir::Component> {
    let ports = as_port_defs(&comp.sig, true);
    let mut component = ir::Component::new(&comp.sig.name.id, ports);
    component.attributes.insert("nointerface", 1);
    let builder = ir::Builder::new(&mut component, lib).not_generated();
    let mut ctx = Context::new(sigs, builder);
    for cmd in comp.body {
        compile_command(cmd, &mut ctx);
    }
    Ok(component)
}

fn compile_signature(sig: &ast::Signature) -> ir::Primitive {
    ir::Primitive {
        name: sig.name.id.clone().into(),
        params: Vec::new(),
        signature: as_port_defs(sig, false)
            .into_iter()
            .map(
                |ir::PortDef {
                     name,
                     width,
                     direction,
                     attributes,
                 }| ir::PortDef {
                    name,
                    width: ir::Width::Const { value: width },
                    direction,
                    attributes,
                },
            )
            .collect(),
        is_comb: false,
        attributes: ir::Attributes::default(),
    }
}

fn init_calyx(
    lib_loc: &Path,
    externs: &[(String, Vec<ast::Signature>)],
) -> CalyxResult<ir::Context> {
    let mut prims = PathBuf::from(lib_loc);
    prims.push("primitives");
    prims.push("core.futil");
    let mut ws = frontend::Workspace::construct(&Some(prims), lib_loc)?;
    // Add externals
    ws.externs.extend(externs.iter().map(|(file, sigs)| {
        (
            PathBuf::from(file),
            sigs.iter().map(compile_signature).collect(),
        )
    }));

    // define a fake main component
    let main = frontend::ast::ComponentDef::new("main", vec![]);
    ws.components.push(main);
    let mut ctx = ir::from_ast::ast_to_ir(ws)?;
    ctx.components.retain(|c| c.name != "main");
    Ok(ctx)
}

fn print(ctx: ir::Context) -> FilamentResult<()> {
    let mut out = &mut std::io::stdout();
    for (path, prims) in ctx.lib.externs() {
        ir::Printer::write_extern(
            (&path, &prims.into_iter().map(|(_, v)| v).collect_vec()),
            &mut out,
        )?;
    }
    for comp in &ctx.components {
        ir::Printer::write_component(comp, &mut out)?;
        println!();
    }
    Ok(())
}

pub fn compile(ns: ast::Namespace, opts: &Opts) -> FilamentResult<()> {
    let mut calyx_ctx = init_calyx(&opts.calyx_primitives, &ns.externs)
        .map_err(|err| Error::misc(format!("{:?}", err)))?;

    let sigs = ns
        .externs
        .iter()
        .flat_map(|(_, comps)| {
            comps
                .iter()
                .map(|s| (s.name.clone(), as_port_defs(s, false)))
        })
        .collect::<HashMap<_, _>>();

    for comp in ns.components {
        calyx_ctx.components.push(compile_component(
            comp,
            &sigs,
            &calyx_ctx.lib,
        )?);
    }

    print(calyx_ctx)
}
