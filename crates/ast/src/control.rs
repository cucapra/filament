use super::{
    Binding, Expr, Id, Implication, Loc, OrderConstraint, Range, Time,
};
use fil_utils::PortAttrs;
use struct_variant::struct_variant;

#[derive(Clone, Debug)]
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

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // For now, always show as range. Could add logic to detect single indices.
        write!(f, "{}:{}", self.start, self.end)
    }
}

/// Unified port representation
#[derive(Clone, Debug)]
pub struct Port {
    /// Base port reference
    pub base: PortRef,
    /// Optional array/bundle access
    pub access: Vec<Loc<Access>>,
    /// Source location for error reporting
    pub loc: fil_utils::GPosIdx,
}

/// Base port reference
#[derive(Clone, Debug, PartialEq)]
pub enum PortRef {
    /// Reference to a port on 'this' component
    This { port: Loc<Id> },
    /// Reference to a port on an instance or invoke
    Instance { instance: Loc<Id>, port: Loc<Id> },
}

impl Port {
    /// Create a simple port reference (this.port)
    pub fn this_port(port: Loc<Id>) -> Self {
        let loc = port.pos();
        Port {
            base: PortRef::This { port },
            access: Vec::new(),
            loc,
        }
    }

    /// Create an instance port reference (inst.port)
    pub fn instance_port(instance: Loc<Id>, port: Loc<Id>) -> Self {
        let loc = instance.pos(); // Using first location, could union if needed
        Port {
            base: PortRef::Instance { instance, port },
            access: Vec::new(),
            loc,
        }
    }

    /// Add array/bundle access to a port
    pub fn with_access(mut self, access: Vec<Loc<Access>>) -> Self {
        self.access = access;
        self
    }

    /// Check if this is a bundle access
    pub fn is_bundle(&self) -> bool {
        !self.access.is_empty()
    }

    /// Get the instance name if this is an instance port
    pub fn instance(&self) -> Option<&Loc<Id>> {
        match &self.base {
            PortRef::Instance { instance, .. } => Some(instance),
            _ => None,
        }
    }

    /// Get the port name
    pub fn port_name(&self) -> &Loc<Id> {
        match &self.base {
            PortRef::This { port } => port,
            PortRef::Instance { port, .. } => port,
        }
    }

    // Legacy constructors for backwards compatibility during migration
    pub fn inv_port(comp: Loc<Id>, name: Loc<Id>) -> Self {
        Self::instance_port(comp, name)
    }

    pub fn this(p: Loc<Id>) -> Self {
        Self::this_port(p)
    }

    pub fn bundle(name: Loc<Id>, access: Vec<Loc<Access>>) -> Self {
        Self::this_port(name).with_access(access)
    }

    pub fn inv_bundle(
        invoke: Loc<Id>,
        port: Loc<Id>,
        access: Vec<Loc<Access>>,
    ) -> Self {
        Self::instance_port(invoke, port).with_access(access)
    }

    pub fn resolve_exprs(mut self, bindings: &Binding<Expr>) -> Self {
        if !self.access.is_empty() {
            self.access = self
                .access
                .into_iter()
                .map(|i| i.map(|i| i.resolve(bindings)))
                .collect();
        }
        self
    }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.base {
            PortRef::This { port } => write!(f, "{}", port),
            PortRef::Instance { instance, port } => {
                write!(f, "{}.{}", instance, port)
            }
        }?;

        for access in &self.access {
            write!(f, "{{{}}}", access)?;
        }

        Ok(())
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
    /// Attributes associated with the bundle
    pub attrs: PortAttrs,
}

impl Bundle {
    pub fn new(name: Loc<Id>, typ: BundleType, attrs: PortAttrs) -> Self {
        Self { name, typ, attrs }
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
