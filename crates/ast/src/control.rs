use super::{
    Binding, Expr, Id, Implication, Loc, OrderConstraint, Range, Time,
};
use struct_variant::struct_variant;

#[derive(Clone)]
/// Access into a bundle
pub struct Access {
    pub start: Expr,
    pub end: Expr,
}

impl Access {
    pub fn range(start: Expr, end: Expr) -> Self {
        Access { start, end }
    }

    pub fn resolve(self, binds: &Binding<Expr>) -> Self {
        Access {
            start: self.start.resolve(binds),
            end: self.end.resolve(binds),
        }
    }
}

impl From<Expr> for Access {
    fn from(e: Expr) -> Self {
        Access {
            start: e.clone(),
            end: e + Expr::concrete(1),
        }
    }
}

/// A port mentioned in the program
// XXX(rachit): the bundle and non-bundle variants can be unified because
// astconv treats them the same anyways.
#[derive(Clone)]
pub enum Port {
    /// A port on this component
    This(Loc<Id>),
    /// A port on an invoke
    InvPort { invoke: Loc<Id>, name: Loc<Id> },
    /// A port represented by an index into a bundle
    Bundle {
        name: Loc<Id>,
        access: Vec<Loc<Access>>,
    },
    /// A bundle port on an invocation
    InvBundle {
        invoke: Loc<Id>,
        port: Loc<Id>,
        access: Vec<Loc<Access>>,
    },
}

impl Port {
    pub fn inv_port(comp: Loc<Id>, name: Loc<Id>) -> Self {
        Port::InvPort { invoke: comp, name }
    }

    pub fn this(p: Loc<Id>) -> Self {
        Port::This(p)
    }

    pub fn bundle(name: Loc<Id>, access: Vec<Loc<Access>>) -> Self {
        Port::Bundle { name, access }
    }

    pub fn inv_bundle(
        invoke: Loc<Id>,
        port: Loc<Id>,
        access: Vec<Loc<Access>>,
    ) -> Self {
        Port::InvBundle {
            invoke,
            port,
            access,
        }
    }

    pub fn resolve_exprs(self, bindings: &Binding<Expr>) -> Self {
        match self {
            Port::Bundle { name, access } => Port::Bundle {
                name,
                access: access
                    .into_iter()
                    .map(|i| i.map(|i| i.resolve(bindings)))
                    .collect(),
            },
            Port::InvBundle {
                invoke,
                port,
                access,
            } => Port::InvBundle {
                invoke,
                port,
                access: access
                    .into_iter()
                    .map(|i| i.map(|a| a.resolve(bindings)))
                    .collect(),
            },
            _ => self,
        }
    }
}

#[struct_variant]
#[derive(Clone)]
/// Command in a component
pub enum Command {
    Invoke,
    Instance,
    Fact,
    Connect,
    ForLoop,
    ParamLet,
    If,
    Bundle,
    Exists,
}

#[derive(Clone)]
/// A new component instance
pub struct Instance {
    /// Name of the instance.
    pub name: Loc<Id>,
    /// Name of the component
    pub component: Loc<Id>,
    /// Liveness of the instance
    pub lives: Vec<Loc<Range>>,
    /// Bindings provided for this instance
    pub params: Vec<Loc<Expr>>,
}
impl Instance {
    pub fn new(
        name: Loc<Id>,
        component: Loc<Id>,
        params: Vec<Loc<Expr>>,
        lives: Vec<Loc<Range>>,
    ) -> Self {
        Instance {
            name,
            component,
            lives,
            params,
        }
    }
}

#[derive(Clone)]
/// An Invocation
pub struct Invoke {
    /// Name of the variable being assigned
    pub name: Loc<Id>,
    /// Name of the component being invoked
    pub instance: Loc<Id>,
    /// Abstract variables used for this invocation
    pub abstract_vars: Vec<Loc<Time>>,
    /// Assignment for the ports
    pub ports: Vec<Loc<Port>>,
}

impl Invoke {
    pub fn new(
        name: Loc<Id>,
        instance: Loc<Id>,
        abstract_vars: Vec<Loc<Time>>,
        ports: Vec<Loc<Port>>,
    ) -> Self {
        Self {
            name,
            instance,
            abstract_vars,
            ports,
        }
    }

    // XXX: This can probably be removed
    pub fn bindings<I>(&self, abstract_vars: I) -> Binding<Time>
    where
        I: Iterator<Item = Id>,
    {
        Binding::new(
            abstract_vars
                .zip(self.abstract_vars.iter().cloned().map(|t| t.take())),
        )
    }
}

#[derive(Clone)]
/// An `assert` or `assume` statement.
/// Contains a guard
/// If `checked` is true, the statement is checked to be statically true.
/// Otherwise, it is assumed to be true.
pub struct Fact {
    pub cons: Loc<Implication<Expr>>,
    // If this fact is statically checked.
    pub checked: bool,
}

impl Fact {
    pub fn assume(cons: Loc<Implication<Expr>>) -> Self {
        Fact {
            cons,
            checked: false,
        }
    }

    pub fn assert(cons: Loc<Implication<Expr>>) -> Self {
        Fact {
            cons,
            checked: true,
        }
    }

    pub fn exprs(&self) -> Vec<&Expr> {
        self.cons.inner().exprs()
    }

    /// Resolve expression in the assumption
    pub fn resolve(self, bind: &Binding<Expr>) -> Self {
        Fact {
            cons: self.cons.map(|c| c.resolve_expr(bind)),
            ..self
        }
    }
}

#[derive(Clone)]
/// A Connection between ports
pub struct Connect {
    /// Destination port
    pub dst: Loc<Port>,
    /// Source port
    pub src: Loc<Port>,
}

impl Connect {
    pub fn new(dst: Loc<Port>, src: Loc<Port>) -> Self {
        Self { dst, src }
    }
}

#[derive(Clone)]
/// A generative loop:
/// ```fil
/// for i in 0..W { ... }
/// ```
pub struct ForLoop {
    /// Index associated with this loop
    pub idx: Loc<Id>,
    /// Start of the range of this loop
    pub start: Expr,
    /// End of the range of this loop
    pub end: Expr,
    /// Body of the loop
    pub body: Vec<Command>,
}

impl ForLoop {
    pub fn new(
        idx: Loc<Id>,
        start: Expr,
        end: Expr,
        body: Vec<Command>,
    ) -> Self {
        Self {
            idx,
            start,
            end,
            body,
        }
    }
}

#[derive(Clone)]
/// A conditional statement:
/// The `then` branch is checked assuming that the condition is true and the `else` branch is checked
/// assuming that the condition is false.
pub struct If {
    pub cond: OrderConstraint<Expr>,
    pub then: Vec<Command>,
    pub alt: Vec<Command>,
}

impl If {
    pub fn new(
        cond: OrderConstraint<Expr>,
        then: Vec<Command>,
        alt: Vec<Command>,
    ) -> Self {
        Self { cond, then, alt }
    }
}

#[derive(Clone)]
/// The type of the bundle:
/// ```fil
/// for<i> ['G+i, 'G+i+1] W
/// ```
pub struct BundleType {
    /// The name of the parameter for the bundle type
    pub idx: Vec<Loc<Id>>,
    /// Length of the bundle. The index parameter ranges over [0, len)
    pub len: Vec<Loc<Expr>>,
    /// Availability interval for the bundle
    pub liveness: Loc<Range>,
    /// Bitwidth of the bundle
    pub bitwidth: Loc<Expr>,
}

impl BundleType {
    pub fn new(
        idx: Vec<Loc<Id>>,
        len: Vec<Loc<Expr>>,
        liveness: Loc<Range>,
        bitwidth: Loc<Expr>,
    ) -> Self {
        Self {
            idx,
            len,
            liveness,
            bitwidth,
        }
    }

    pub fn resolve_exprs(self, binding: &Binding<Expr>) -> Self {
        Self {
            idx: self.idx,
            len: self
                .len
                .into_iter()
                .map(|e| e.map(|e| e.resolve(binding)))
                .collect(),
            liveness: self.liveness.map(|e| e.resolve_exprs(binding)),
            bitwidth: self.bitwidth.map(|e| e.resolve(binding)),
        }
    }

    pub fn resolve_event(self, binding: &Binding<Time>) -> Self {
        Self {
            liveness: self.liveness.map(|e| e.resolve_event(binding)),
            ..self
        }
    }

    /// Check if this bundle type is alpha equivalent to another bundle type
    pub fn alpha_eq(&self, _: Self) -> bool {
        // Resolve the other expression by providing a binding for index to be
        // the same name as this type's index.
        // let binding = Binding::new(Some((other.idx, Expr::from(self.idx))));
        // let resolved = other.resolve_exprs(&binding);
        todo!()
    }
}

#[derive(Clone)]
/// Represents a bundle of wires with timing guarantees
/// ```fil
/// bundle f[10]: for<i> ['G+i, 'G+i+1] W;
/// ```
pub struct Bundle {
    /// Name of the bundle
    pub name: Loc<Id>,
    /// Type of the bundle
    pub typ: BundleType,
}

impl Bundle {
    pub fn new(name: Loc<Id>, typ: BundleType) -> Self {
        Self { name, typ }
    }

    /// Resolve expressions in the Bundle
    pub fn resolve_exprs(self, binding: &Binding<Expr>) -> Self {
        Self {
            typ: self.typ.resolve_exprs(binding),
            ..self
        }
    }

    /// Resolve events in the Bundle
    pub fn resolve_event(self, binding: &Binding<Time>) -> Self {
        Self {
            typ: self.typ.resolve_event(binding),
            ..self
        }
    }
}

/// A let-bound parameter
#[derive(Clone)]
pub struct ParamLet {
    pub name: Loc<Id>,
    /// The expression for the parameter binding
    pub expr: Option<Expr>,
}

#[derive(Clone)]
/// Binding for an existentially quantified parameter
pub struct Exists {
    /// The existentially quantified parameter
    pub param: Loc<Id>,
    /// The binding expression for the parameter
    pub bind: Loc<Expr>,
}
