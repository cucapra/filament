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
use super::TimeRep;
use crate::cmdline::Opts;
use crate::errors::Error;
use crate::{core, errors::FilamentResult};

// Attribute attached to event signals in a module interface.
const FIL_EVENT: &str = "fil_event";

/// Bindings associated with the current compilation context
#[derive(Default)]
pub struct Binding {
    // External signatures
    sigs: HashMap<core::Id, Vec<ir::PortDef<u64>>>,

    // Component signatures
    comps: HashMap<core::Id, RRC<ir::Cell>>,
}

impl Binding {
    fn cell_to_port_def(cr: &RRC<ir::Cell>) -> Vec<ir::PortDef<u64>> {
        let cell = cr.borrow();
        cell.ports()
            .iter()
            .map(|pr| {
                let port = pr.borrow();
                // Reverse port direction because signature refers to internal interface.
                (port.name.clone(), port.width, port.direction.reverse()).into()
            })
            .collect()
    }

    pub fn get(&self, name: &core::Id) -> Vec<ir::PortDef<u64>> {
        self.sigs
            .get(name)
            .cloned()
            .or_else(|| self.comps.get(name).map(Binding::cell_to_port_def))
            .unwrap_or_else(|| panic!("No binding for {name}"))
    }

    pub fn insert_comp(&mut self, name: core::Id, sig: RRC<ir::Cell>) {
        self.comps.insert(name, sig);
    }
}

pub struct Context<'a> {
    /// Builder for the current component
    pub builder: ir::Builder<'a>,

    /// Mapping from names to signatures for components and externals.
    binding: &'a Binding,

    /// Mapping from instances to cells
    instances: HashMap<core::Id, RRC<ir::Cell>>,

    /// Mapping from invocation name to instance
    invokes: HashMap<core::Id, RRC<ir::Cell>>,

    /// Mapping from name to FSMs
    fsms: HashMap<core::Id, Fsm>,
}

impl Context<'_> {
    pub fn compile_port(
        &mut self,
        port: &core::Port,
    ) -> (RRC<ir::Port>, Option<ir::Guard>) {
        match port {
            core::Port::ThisPort(p) => {
                let this = self.builder.component.signature.borrow();
                (this.get(p), None)
            }
            core::Port::CompPort { comp, name } => {
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
            core::Port::Constant(c) => {
                let cr = self.builder.add_constant(*c, 32);
                let c = cr.borrow();
                (c.get("out"), None)
            }
        }
    }
    fn get_sig(&self, comp: &core::Id) -> Vec<ir::PortDef<u64>> {
        self.binding.get(comp)
    }

    fn add_invoke(&mut self, inv: core::Id, comp: core::Id) {
        let cell = &self
            .instances
            .get(&comp)
            .unwrap_or_else(|| panic!("Unknown instance: {}", comp));
        self.invokes.insert(inv, Rc::clone(cell));
    }
}

impl<'a> Context<'a> {
    fn new(binding: &'a Binding, builder: ir::Builder<'a>) -> Self {
        Context {
            binding,
            builder,
            instances: HashMap::default(),
            invokes: HashMap::default(),
            fsms: HashMap::default(),
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
        core::Command::Fsm(fsm) => {
            let name = fsm.name.clone();
            let f = Fsm::new(fsm, ctx);
            ctx.fsms.insert(name, f);
        }
        core::Command::Invoke(core::Invoke {
            bind,
            instance,
            ports,
            ..
        }) => {
            assert!(
                ports.is_none(),
                "Cannot compile high-level invoke statements"
            );
            ctx.add_invoke(bind, instance);
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

fn as_port_defs(
    sig: &core::Signature<TimeRep>,
    extend: bool,
) -> Vec<ir::PortDef<u64>> {
    let mut ports: Vec<ir::PortDef<u64>> = sig
        .inputs
        .iter()
        .map(|pd| {
            (
                ir::Id::from(pd.name.id.clone()),
                pd.bitwidth,
                ir::Direction::Input,
            )
                .into()
        })
        .chain(sig.interface_signals.iter().map(|id| {
            let mut pd = ir::PortDef::from((
                ir::Id::from(id.name.id.clone()),
                1,
                ir::Direction::Input,
            ));
            pd.attributes.insert(FIL_EVENT, 1);
            pd
        }))
        .chain(sig.outputs.iter().map(|pd| {
            (
                ir::Id::from(pd.name.id.clone()),
                pd.bitwidth,
                ir::Direction::Output,
            )
                .into()
        }))
        .chain(sig.unannotated_ports.iter().map(|(n, bw)| {
            let mut pd =
                ir::PortDef::from((n.id.clone(), *bw, ir::Direction::Input));
            if INTERFACE_PORTS.iter().any(|(n, _, _)| n == &pd.name.id) {
                pd.attributes.insert(pd.name.clone(), 1)
            }
            pd
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
    comp: core::Component<TimeRep>,
    sigs: &Binding,
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

fn compile_signature(sig: &core::Signature<TimeRep>) -> ir::Primitive {
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
    externs: &[(String, Vec<core::Signature<TimeRep>>)],
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

pub fn compile(
    ns: core::Namespace<TimeRep>,
    opts: &Opts,
) -> FilamentResult<()> {
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

    let mut bindings = Binding {
        sigs,
        ..Default::default()
    };

    for comp in ns.components {
        let comp = compile_component(comp, &bindings, &calyx_ctx.lib)?;
        bindings.insert_comp(
            core::Id::from(comp.name.id.as_str()),
            Rc::clone(&comp.signature),
        );
        calyx_ctx.components.push(comp);
    }

    print(calyx_ctx)
}
