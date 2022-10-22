use super::Fsm;
use crate::{
    cmdline::Opts,
    errors::{Error, FilamentResult},
    event_checker::ast,
};
use calyx::{
    errors::CalyxResult,
    frontend,
    ir::{self, RRC},
    structure,
};
use itertools::Itertools;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::path::{Path, PathBuf};
use std::rc::Rc;

/// Attribute attached to event signals in a module interface.
const FIL_EVENT: &str = "fil_event";
/// Attribute that represents the delay of a particular event
const DELAY: &str = "delay";

/// Bindings associated with the current compilation context
#[derive(Default)]
pub struct Binding {
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

    /// Get a binding associated with a name
    pub fn get(&self, name: &ast::Id) -> Option<Vec<ir::PortDef<u64>>> {
        self.comps.get(name).map(Self::cell_to_port_def)
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
            ast::PortType::InvPort { invoke: comp, name } => {
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
    fn get_sig(&self, comp: &ast::Id) -> Option<Vec<ir::PortDef<u64>>> {
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
            ast::PortType::InvPort { invoke: comp, name } => {
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
    let mut comp = ir::Component::new(
        ir::Id::from(format!("fsm_{}", states)),
        ports,
        false,
    );
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

fn compile_connect(con: ast::Connect, ctx: &mut Context) {
    let ast::Connect {
        dst, src, guard, ..
    } = con;

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

fn as_port_defs<FW, CW, F0, F1>(
    // The signature to be converted
    sig: &ast::Signature<FW>,
    // Transformation for ports that may have parametric width.
    port_transform: F0,
    // Transformation for ports that have a concrete width (interface ports, clk, reset)
    concrete_transform: F1,
    // Is this a component or external
    is_comp: bool,
) -> Vec<ir::PortDef<CW>>
where
    FW: Clone,
    F0: Fn(&ast::PortDef<FW>, ir::Direction) -> ir::PortDef<CW>,
    F1: Fn(&ast::Id, u64) -> ir::PortDef<CW>,
{
    let mut ports: Vec<ir::PortDef<CW>> = sig
        .inputs
        .iter()
        .map(|pd| port_transform(pd, ir::Direction::Input))
        .chain(sig.interface_signals.iter().filter(|id| !id.phantom).map(
            |id| {
                let mut pd = concrete_transform(&id.name, 1);
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
            },
        ))
        .chain(
            sig.outputs
                .iter()
                .map(|pd| port_transform(pd, ir::Direction::Output)),
        )
        .chain(
            sig.unannotated_ports
                .iter()
                .map(|(n, bw)| concrete_transform(n, *bw)),
        )
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
            let mut pd = concrete_transform(&name.into(), 1);
            pd.attributes.insert(name, 1);
            ports.push(pd);
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
    let port_transform =
        |pd: &ast::PortDef<u64>, dir: ir::Direction| -> ir::PortDef<u64> {
            let mut pd: ir::PortDef<u64> =
                (pd.name.id().clone(), pd.bitwidth, dir).into();
            pd.attributes.insert("data", 1);
            pd
        };
    let concrete_transform = |name: &ast::Id, width: u64| -> ir::PortDef<u64> {
        (name.id().clone(), width, ir::Direction::Input).into()
    };
    let ports =
        as_port_defs(&comp.sig, port_transform, concrete_transform, true);
    let mut component = ir::Component::new(&comp.sig.name.id(), ports, false);
    component.attributes.insert("nointerface", 1);
    let builder = ir::Builder::new(&mut component, lib).not_generated();
    let mut ctx = Context::new(sigs, builder, lib);

    let mut cons = vec![];

    // Construct bindings
    for cmd in comp.body {
        match cmd {
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
            ast::Command::Fsm(fsm) => {
                // If FSM with required number of states has not been constructed, define a new component for it
                if !ctx.binding.fsm_comps.contains_key(&fsm.states) {
                    define_fsm_component(fsm.states, &mut ctx);
                }
                // Construct the FSM
                let name = fsm.name.clone();
                let f = Fsm::new(&fsm, &mut ctx);
                ctx.fsms.insert(name, f);
            }
            ast::Command::Instance(ast::Instance {
                name,
                component,
                bindings,
                ..
            }) => {
                let cell = if let Some(sig) = ctx.get_sig(&component) {
                    ctx.builder.add_component(
                        name.id().clone(),
                        component.id().clone(),
                        sig,
                    )
                } else {
                    ctx.builder.add_primitive(
                        name.id().clone(),
                        component.id(),
                        &bindings,
                    )
                };
                cell.borrow_mut().attributes.insert("data", 1);
                ctx.instances.insert(name, cell);
            }
            ast::Command::Connect(con) => {
                cons.push(con);
            }
        };
    }
    // Compile commands
    cons.into_iter()
        .for_each(|con| compile_connect(con, &mut ctx));
    Ok(component)
}

fn prim_as_port_defs(
    sig: &ast::Signature<ast::PortParam>,
) -> Vec<ir::PortDef<ir::Width>> {
    let port_transform = |pd: &ast::PortDef<ast::PortParam>,
                          dir: ir::Direction|
     -> ir::PortDef<ir::Width> {
        let width = match &pd.bitwidth {
            ast::PortParam::Const(v) => ir::Width::Const { value: *v },
            ast::PortParam::Var(v) => ir::Width::Param {
                value: v.id().clone().into(),
            },
        };
        let mut attributes = ir::Attributes::default();
        attributes.insert("data", 1);
        ir::PortDef {
            name: ir::Id::from(pd.name.id().as_ref()),
            direction: dir,
            width,
            attributes,
        }
    };
    let concrete_transform =
        |name: &ast::Id, bw: u64| -> ir::PortDef<ir::Width> {
            ir::PortDef {
                name: ir::Id::from(name.id().as_ref()),
                direction: ir::Direction::Input,
                width: ir::Width::Const { value: bw },
                attributes: Default::default(),
            }
        };
    as_port_defs(sig, port_transform, concrete_transform, false)
}

fn compile_signature(sig: &ast::Signature<ast::PortParam>) -> ir::Primitive {
    ir::Primitive {
        name: sig.name.id().clone().into(),
        params: sig.params.iter().map(|p| p.id().clone().into()).collect(),
        signature: prim_as_port_defs(sig),
        is_comb: false,
        attributes: ir::Attributes::default(),
    }
}

fn init_calyx(
    lib_loc: &Path,
    externs: &[(String, Vec<ast::Signature<ast::PortParam>>)],
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
    let main = frontend::ast::ComponentDef::new("main", false, vec![]);
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

    let mut bindings = Binding::default();

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
