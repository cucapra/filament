use calyx::errors::CalyxResult;
use calyx::frontend;
use calyx::ir;
use calyx::ir::RRC;
use calyx::structure;
use itertools::Itertools;
use std::collections::HashMap;
use std::collections::HashSet;
use std::iter;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use super::Fsm;
use crate::cmdline::Opts;
use crate::errors::Error;
use crate::{errors::FilamentResult, event_checker::ast};

/// Attribute attached to event signals in a module interface.
const FIL_EVENT: &str = "fil_event";
/// Attribute that represents the delay of a particular event
const DELAY: &str = "delay";

/// Bindings associated with the current compilation context
#[derive(Default)]
pub struct Binding {
    // External signatures
    sigs: HashMap<ast::Id, Vec<ir::PortDef<u64>>>,

    // Component signatures
    comps: HashMap<ast::Id, RRC<ir::Cell>>,

    /// Mapping to the component representing FSM with partiular number of states
    pub fsm_comps: HashMap<u64, ir::Component>,
}

impl Binding {
    pub fn cell_to_port_def(cr: &RRC<ir::Cell>) -> Vec<ir::PortDef<u64>> {
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

    pub fn get(&self, name: &ast::Id) -> Vec<ir::PortDef<u64>> {
        self.sigs
            .get(name)
            .cloned()
            .or_else(|| self.comps.get(name).map(Binding::cell_to_port_def))
            .unwrap_or_else(|| panic!("No binding for {name}"))
    }

    pub fn insert_comp(&mut self, name: ast::Id, sig: RRC<ir::Cell>) {
        self.comps.insert(name, sig);
    }
}

pub struct Context<'a> {
    /// Builder for the current component
    pub builder: ir::Builder<'a>,

    /// Library signatures
    pub lib: &'a ir::LibrarySignatures,

    /// Mapping from names to signatures for components and externals.
    pub binding: &'a mut Binding,

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
                    (c.get("out"), Some(fsm.event(name)))
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
    fn get_sig(&self, comp: &ast::Id) -> Vec<ir::PortDef<u64>> {
        self.binding.get(comp)
    }

    fn add_invoke(&mut self, inv: ast::Id, comp: ast::Id) {
        let cell = &self
            .instances
            .get(&comp)
            .unwrap_or_else(|| panic!("Unknown instance: {}", comp));
        self.invokes.insert(inv, Rc::clone(cell));
    }
}

impl<'a> Context<'a> {
    fn new(
        binding: &'a mut Binding,
        builder: ir::Builder<'a>,
        lib: &'a ir::LibrarySignatures,
    ) -> Self {
        Context {
            binding,
            builder,
            lib,
            instances: HashMap::default(),
            invokes: HashMap::default(),
            fsms: HashMap::default(),
        }
    }
}

fn compile_guard(guard: ast::Guard, ctx: &mut Context) -> ir::Guard {
    match guard {
        ast::Guard::Or(g1, g2, _) => {
            let c1 = compile_guard(*g1, ctx);
            let c2 = compile_guard(*g2, ctx);
            c1 | c2
        }
        ast::Guard::Port(p) => match &p.typ {
            ast::PortType::ThisPort(p) => {
                let this = ctx.builder.component.signature.borrow();
                this.get(p).into()
            }
            ast::PortType::CompPort { comp, name } => {
                if let Some(fsm) = ctx.fsms.get(comp) {
                    fsm.event(name)
                } else {
                    let cell = ctx.invokes[comp].borrow();
                    cell.get(name).into()
                }
            }
            ast::PortType::Constant(_) => {
                unreachable!("Constants cannot be in guards")
            }
        },
    }
}

/// Construct a new component that represents an FSM with `states`.
/// component fsm_<states>(go: 1) -> (_0: 1, ..., <states-1>: 1) { ... }
fn define_fsm_component(states: u64, ctx: &mut Context) {
    let ports: Vec<ir::PortDef<u64>> = (0..states)
        .map(|n| {
            (ir::Id::from(format!("_{n}")), 1, ir::Direction::Output).into()
        })
        .chain(INTERFACE_PORTS.iter().map(|pd| {
            let mut pd = ir::PortDef::from(pd.clone());
            pd.attributes.insert(&pd.name, 1);
            pd
        }))
        .chain(iter::once(
            (ir::Id::from("go"), 1, ir::Direction::Input).into(),
        ))
        .collect();
    let mut comp =
        ir::Component::new(ir::Id::from(format!("fsm_{}", states)), ports);
    comp.attributes.insert("nointerface", 1);
    let mut builder = ir::Builder::new(&mut comp, ctx.lib).not_generated();

    // Add n-1 registers
    let regs = (0..states - 1)
        .map(|_| builder.add_primitive("r", "std_reg", &[1]))
        .collect_vec();

    // Constant signal
    structure!(builder;
        let signal_on = constant(1, 1);
    );
    // This component's interface
    let this = builder.component.signature.borrow();

    // _0 = go;
    let assign = builder.build_assignment(
        this.get("_0"),
        this.get("go"),
        ir::Guard::True,
    );
    builder.component.continuous_assignments.push(assign);

    // For each register, add the following assignments:
    // rn.write_en = 1'd1;
    // rn.in = r{n-1}.out;
    // _n = rn.out;
    for idx in 0..states - 1 {
        let cell = regs[idx as usize].borrow();
        let write_assign = if idx == 0 {
            builder.build_assignment(
                cell.get("in"),
                this.get("go"),
                ir::Guard::True,
            )
        } else {
            let prev_cell = regs[(idx - 1) as usize].borrow();
            builder.build_assignment(
                cell.get("in"),
                prev_cell.get("out"),
                ir::Guard::True,
            )
        };
        let enable = builder.build_assignment(
            cell.get("write_en"),
            signal_on.borrow().get("out"),
            ir::Guard::True,
        );
        let out = builder.build_assignment(
            this.get(format!("_{}", idx + 1)),
            cell.get("out"),
            ir::Guard::True,
        );
        builder.component.continuous_assignments.extend([
            write_assign,
            enable,
            out,
        ]);
    }
    drop(this);
    ctx.binding.fsm_comps.insert(states, comp);
}

fn compile_command(cmd: ast::Command, ctx: &mut Context) {
    match cmd {
        ast::Command::Fsm(fsm) => {
            // If FSM with required number of states has not been constructed, define a new component for it
            if !ctx.binding.fsm_comps.contains_key(&fsm.states) {
                define_fsm_component(fsm.states, ctx);
            }
            // Construct the FSM
            let name = fsm.name.clone();
            let f = Fsm::new(&fsm, ctx);
            ctx.fsms.insert(name, f);
        }
        ast::Command::Invoke(ast::Invoke {
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
        ast::Command::Instance(ast::Instance {
            name, component, ..
        }) => {
            let cell = ctx.builder.add_component(
                name.id.clone(),
                component.id.clone(),
                ctx.get_sig(&component),
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

fn as_port_defs(sig: &ast::Signature, is_comp: bool) -> Vec<ir::PortDef<u64>> {
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
            if is_comp {
                pd.attributes.insert(
                    DELAY,
                    id.delay().concrete().unwrap_or_else(|| {
                        panic!(
                            "Event does not have a concrete delay: {}",
                            id.delay()
                        )
                    }),
                );
            }
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
    if is_comp {
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
    sigs: &mut Binding,
    lib: &ir::LibrarySignatures,
) -> FilamentResult<ir::Component> {
    let ports = as_port_defs(&comp.sig, true);
    let mut component = ir::Component::new(&comp.sig.name.id, ports);
    component.attributes.insert("nointerface", 1);
    let builder = ir::Builder::new(&mut component, lib).not_generated();
    let mut ctx = Context::new(sigs, builder, lib);
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

    let mut bindings = Binding {
        sigs,
        ..Default::default()
    };

    for comp in ns.components {
        let comp = compile_component(comp, &mut bindings, &calyx_ctx.lib)?;
        bindings.insert_comp(
            ast::Id::from(comp.name.id.as_str()),
            Rc::clone(&comp.signature),
        );
        calyx_ctx.components.push(comp);
    }
    calyx_ctx
        .components
        .extend(bindings.fsm_comps.into_values());

    print(calyx_ctx)
}
