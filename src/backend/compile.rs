use std::collections::HashMap;

use super::verilog;
use crate::{core, errors::FilamentResult};

/// Representation of a Verilog module signature
#[derive(Default)]
struct CompiledSig {
    // Mapping from abstract variables in the signature to ports on the verilog
    // module
    pub abs_vars: HashMap<core::Id, verilog::PortDef>,
    // Mapping from ports to their name in the verilog module
    pub ports: HashMap<core::Id, verilog::PortDef>,
}
impl CompiledSig {
    /// Generate a new name that does not conflict with any of the existing ports
    fn gen_name(&self, name: core::Id) -> core::Id {
        if self.ports.contains_key(&name) || self.abs_vars.contains_key(&name) {
            self.gen_name(core::Id {
                id: format!("{}0", name.id),
            })
        } else {
            name
        }
    }

    pub fn add_abs_var(&mut self, abs: core::Id) {
        self.abs_vars.insert(
            abs.clone(),
            verilog::PortDef {
                name: self.gen_name(abs),
                size: 1,
                direction: verilog::PortDir::Input,
            },
        );
    }
}

/// Representation of an FSM
struct Fsm {
    name: core::Id,
}

impl Fsm {
    fn new(name: core::Id) -> Self {
        Self { name }
    }
}

/// Compilation context
#[derive(Default)]
struct Context {
    pub sigs: HashMap<core::Id, CompiledSig>,
}

struct CompileContext {
    // Mapping for FSMs instantiated with the component's abstract parameters
    pub fsms: HashMap<core::Id, Fsm>,
    // Mapping from invokes to instances
    pub invokes: HashMap<core::Id, core::Id>,
}

fn compile_command(command: core::Command, ctx: &mut CompileContext) {
    match command {
        core::Command::Instance(core::Instance { name, component }) => {}
        core::Command::Invoke(core::Invoke { bind, rhs }) => {
            let instance = &ctx.invokes[&bind];
            todo!();
        }
        core::Command::When(_) => todo!(),
        core::Command::Connect(_) => todo!(),
    }
}

fn compile_component(comp: core::Component) -> FilamentResult<()> {
    let fsms = comp
        .sig
        .abstract_vars
        .iter()
        .map(|abs| (abs.clone(), Fsm::new(abs.clone())))
        .collect();

    let mut ctx = CompileContext {
        fsms,
        invokes: HashMap::default(),
    };

    for command in comp.body {
        compile_command(command, &mut ctx);
    }
    Ok(())
}

pub fn compile(ns: core::Namespace) -> FilamentResult<()> {
    let mut ctx = Context::default();

    // first compile each signature to the equivalent Verilog module signature
    for core::Signature {
        name,
        abstract_vars,
        inputs,
        outputs,
    } in ns.signatures
    {
        let mut csig = CompiledSig::default();
        csig.ports.extend(inputs.into_iter().map(|pd| {
            (
                pd.name.clone(),
                verilog::PortDef {
                    name: pd.name,
                    size: pd.bitwidth,
                    direction: verilog::PortDir::Input,
                },
            )
        }));
        csig.ports.extend(outputs.into_iter().map(|pd| {
            (
                pd.name.clone(),
                verilog::PortDef {
                    name: pd.name,
                    size: pd.bitwidth,
                    direction: verilog::PortDir::Output,
                },
            )
        }));
        abstract_vars
            .into_iter()
            .for_each(|abs| csig.add_abs_var(abs));
        ctx.sigs.insert(name, csig);
    }

    // compile the body of each component
    todo!()
}
