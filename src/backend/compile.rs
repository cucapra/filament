use std::collections::HashMap;

use super::verilog;
use crate::{core, errors::FilamentResult};
use vast::v17::ast as v;

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
                width: 1,
                direction: verilog::PortDir::Input,
            },
        );
    }

    fn all_ports(
        &self,
    ) -> impl Iterator<Item = (&core::Id, &verilog::PortDef)> {
        self.abs_vars.iter().chain(self.ports.iter())
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
struct Context<'a> {
    pub sigs: &'a HashMap<core::Id, CompiledSig>,
    /// Mapping for FSMs instantiated with the component's abstract parameters
    pub fsms: HashMap<core::Id, Fsm>,
    /// Mapping from invokes to instances
    pub invokes: HashMap<core::Id, core::Id>,
    /// Mapping from name of an instance to all triggering events
    pub triggers: HashMap<core::Id, FsmIdxs>,
}

fn compile_command(
    command: core::Command,
    module: &mut v::Module,
    ctx: &mut Context,
) {
    match command {
        core::Command::Instance(core::Instance { name, component }) => {
            // Construct a verilog instance using the component's interface.
            let mut instance =
                v::Instance::new(name.as_ref(), component.as_ref());
            // Construct wires and connect to each port of the instance
            let sig = &ctx.sigs[&component];
            for (port_name, pd) in sig.all_ports() {
                let wire_name = format!("{}_{}", name, port_name);
                module
                    .add_decl(v::Decl::new_logic(wire_name.clone(), pd.width));
                instance.connect(name.as_ref(), v::Expr::Ref(wire_name))
            }
        }
        core::Command::Invoke(core::Invoke {
            bind,
            rhs:
                core::Invocation {
                    comp,
                    abstract_vars,
                    ports,
                    ..
                },
        }) => {
            ctx.invokes.insert(bind, comp);
        }
        core::Command::When(_) => todo!(),
        core::Command::Connect(_) => todo!(),
    }
}

fn compile_component(
    comp: core::Component,
    sigs: &HashMap<core::Id, CompiledSig>,
) -> v::Module {
    let fsms = comp
        .sig
        .abstract_vars
        .iter()
        .map(|abs| (abs.clone(), Fsm::new(abs.clone())))
        .collect();

    let mut ctx = Context {
        sigs,
        fsms,
        invokes: HashMap::default(),
        triggers: HashMap::default(),
    };

    let mut module = v::Module::new(comp.sig.name.as_ref());
    for command in comp.body {
        compile_command(command, &mut module, &mut ctx);
    }
    module
}

/// Represents a state in an FSM.
struct FsmIdx {
    pub name: core::Id,
    pub state: u64,
}

impl FsmIdx {
    fn new(name: core::Id, state: u64) -> Self {
        Self { name, state }
    }
}

/// An interval time expression that denotes a max of sums expression.
type FsmIdxs = Vec<FsmIdx>;

/// Reduces an IntervalTime expression into a max of sums representation.
/// The returned vector represents all the non-max IntervalTime expressions of
/// which the max is being computed.
fn max_of_sums(event: core::IntervalTime, acc: &mut FsmIdxs) {
    use self::core::{IntervalTime::*, TimeOp::*};
    match event {
        Abstract(name) => acc.push(FsmIdx::new(name, 0)),
        Concrete(_) => {
            panic!("Concrete interval time reached while computing max of sums")
        }
        BinOp {
            op: Max,
            left,
            right,
        } => {
            max_of_sums(*left, acc);
            max_of_sums(*right, acc);
        }
        BinOp {
            op: Add,
            left,
            right,
        } => {
            match (*left, *right) {
                (Concrete(n), e) | (e, Concrete(n)) => {
                    match e {
                        Abstract(name) => acc.push(FsmIdx::new(name, n)),
                        BinOp { op: Max, left, right } => {
                            let left_sum = core::IntervalTime::binop_add(*left, core::IntervalTime::concrete(n));
                            max_of_sums(left_sum, acc);
                            let right_sum = core::IntervalTime::binop_add(*right, core::IntervalTime::concrete(n));
                            max_of_sums(right_sum, acc);
                        }
                        BinOp { op: Add, .. } => panic!("Add expressions are nested, should've been reduced"),
                        Concrete(_) => panic!("Event add expression is sum of two values, should've been reduced already")
                    }
                }
                _ => panic!("Event add expression does not have a nat")
            }
        }
    }
}

pub fn compile(ns: core::Namespace) -> FilamentResult<()> {
    let mut sigs = HashMap::new();

    // first compile each signature to the equivalent Verilog module signature
    for core::Signature {
        name,
        abstract_vars,
        inputs,
        outputs,
        ..
    } in ns.signatures
    {
        let mut csig = CompiledSig::default();
        csig.ports.extend(inputs.into_iter().map(|pd| {
            (
                pd.name.clone(),
                verilog::PortDef {
                    name: pd.name,
                    width: pd.bitwidth,
                    direction: verilog::PortDir::Input,
                },
            )
        }));
        csig.ports.extend(outputs.into_iter().map(|pd| {
            (
                pd.name.clone(),
                verilog::PortDef {
                    name: pd.name,
                    width: pd.bitwidth,
                    direction: verilog::PortDir::Output,
                },
            )
        }));
        abstract_vars
            .into_iter()
            .for_each(|abs| csig.add_abs_var(abs));
        sigs.insert(name, csig);
    }

    // compile the body of each component
    for comp in ns.components {
        compile_component(comp, &sigs);
    }
    todo!()
}
