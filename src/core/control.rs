use super::{Expr, Id, Loc, OrderConstraint, PortDef, Range, Time};
use crate::utils::Binding;
use crate::{
    errors::{Error, FilamentResult},
    utils::GPosIdx,
};
use itertools::Itertools;
use std::fmt::Display;

#[derive(Clone)]
/// Access into a bundle
pub enum Access {
    /// Index into the bundle
    Index(Expr),
    /// Range over a bundle
    Range { start: Expr, end: Expr },
}

impl Access {
    pub fn range(start: Expr, end: Expr) -> Self {
        Access::Range { start, end }
    }

    pub fn resolve(self, binds: &Binding<Expr>) -> Self {
        match self {
            Access::Index(e) => Access::Index(e.resolve(binds)),
            Access::Range { start, end } => Access::Range {
                start: start.resolve(binds),
                end: end.resolve(binds),
            },
        }
    }
}

impl From<Expr> for Access {
    fn from(e: Expr) -> Self {
        Access::Index(e)
    }
}

impl std::fmt::Display for Access {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Access::Index(e) => write!(f, "{e}"),
            Access::Range { start, end } => write!(f, "{start}..{end}"),
        }
    }
}

/// A port mentioned in the program
#[derive(Clone)]
pub enum Port {
    /// A port on this component
    This(Loc<Id>),
    /// A constant
    Constant(u64),
    /// A port on an invoke
    InvPort { invoke: Loc<Id>, name: Loc<Id> },

    /// A port represented by an index into a bundle
    Bundle { name: Loc<Id>, access: Loc<Access> },
    /// A bundle port on an invocation
    InvBundle {
        invoke: Loc<Id>,
        port: Loc<Id>,
        access: Loc<Access>,
    },
}

impl Port {
    pub fn inv_port(comp: Loc<Id>, name: Loc<Id>) -> Self {
        Port::InvPort { invoke: comp, name }
    }

    pub fn this(p: Loc<Id>) -> Self {
        Port::This(p)
    }

    pub fn constant(v: u64) -> Self {
        Port::Constant(v)
    }

    pub fn bundle(name: Loc<Id>, access: Loc<Access>) -> Self {
        Port::Bundle { name, access }
    }

    pub fn inv_bundle(
        invoke: Loc<Id>,
        port: Loc<Id>,
        access: Loc<Access>,
    ) -> Self {
        Port::InvBundle {
            invoke,
            port,
            access,
        }
    }

    /// Returns true iff this port represents a ranged access on a bundle
    pub fn range_access(&self) -> bool {
        match self {
            Port::Bundle { access, .. } | Port::InvBundle { access, .. } => {
                matches!(access.inner(), Access::Range { .. })
            }
            _ => false,
        }
    }

    pub fn resolve_exprs(self, bindings: &Binding<Expr>) -> Self {
        match self {
            Port::Bundle { name, access } => Port::Bundle {
                name,
                access: access.map(|i| i.resolve(bindings)),
            },
            Port::InvBundle {
                invoke,
                port,
                access,
            } => Port::InvBundle {
                invoke,
                port,
                access: access.map(|a| a.resolve(bindings)),
            },
            _ => self,
        }
    }
}

impl std::fmt::Display for Port {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Port::This(p) => write!(f, "{}", p),
            Port::InvPort { invoke: comp, name } => {
                write!(f, "{}.{}", comp, name)
            }
            Port::Constant(n) => write!(f, "{}", n),
            Port::Bundle { name, access } => {
                write!(f, "{name}{{{access}}}")
            }
            Port::InvBundle {
                invoke,
                port,
                access,
            } => {
                write!(f, "{invoke}.{port}{{{access}}}")
            }
        }
    }
}

#[derive(Clone)]
/// Command in a component
pub enum Command {
    Invoke(Invoke),
    Instance(Instance),
    Assume(Assume),
    Connect(Connect),
    Fsm(Fsm),
    ForLoop(ForLoop),
    If(If),
    Bundle(Bundle),
}

impl From<Fsm> for Command {
    fn from(v: Fsm) -> Self {
        Self::Fsm(v)
    }
}
impl From<Connect> for Command {
    fn from(v: Connect) -> Self {
        Self::Connect(v)
    }
}
impl From<Instance> for Command {
    fn from(v: Instance) -> Self {
        Self::Instance(v)
    }
}
impl From<Invoke> for Command {
    fn from(v: Invoke) -> Self {
        Self::Invoke(v)
    }
}
impl From<Bundle> for Command {
    fn from(v: Bundle) -> Self {
        Self::Bundle(v)
    }
}
impl From<Assume> for Command {
    fn from(v: Assume) -> Self {
        Self::Assume(v)
    }
}

impl Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Command::Invoke(inv) => write!(f, "{}", inv),
            Command::Instance(ins) => write!(f, "{}", ins),
            Command::Connect(con) => write!(f, "{}", con),
            Command::Fsm(fsm) => write!(f, "{}", fsm),
            Command::ForLoop(l) => write!(f, "{}", l),
            Command::If(l) => write!(f, "{}", l),
            Command::Bundle(b) => write!(f, "{b}"),
            Command::Assume(a) => write!(f, "{a}"),
        }
    }
}

#[derive(Clone)]
/// A new component instance
pub struct Instance {
    /// Name of the instance.
    pub name: Loc<Id>,
    /// Name of the component
    pub component: Loc<Id>,
    /// Bindings provided for this instance
    pub bindings: Vec<Loc<Expr>>,
}
impl Instance {
    pub fn new(
        name: Loc<Id>,
        component: Loc<Id>,
        bindings: Vec<Loc<Expr>>,
    ) -> Self {
        Instance {
            name,
            component,
            bindings,
        }
    }
}
impl std::fmt::Display for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} := new {}", self.name, self.component)?;
        if !self.bindings.is_empty() {
            write!(f, "[{}]", self.bindings.iter().join(", "))
        } else {
            Ok(())
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
    pub ports: Option<Vec<Loc<Port>>>,
}

impl Invoke {
    pub fn new(
        name: Loc<Id>,
        instance: Loc<Id>,
        abstract_vars: Vec<Loc<Time>>,
        ports: Option<Vec<Loc<Port>>>,
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
impl Display for Invoke {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let abs = self
            .abstract_vars
            .iter()
            .map(|it| format!("{}", it))
            .collect::<Vec<String>>()
            .join(",");
        if let Some(ports) = &self.ports {
            write!(
                f,
                "{} := {}<{}>({})",
                self.name,
                self.instance,
                abs,
                ports
                    .iter()
                    .map(|port| port.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        } else {
            write!(f, "{} := invoke {}<{}>", self.name, self.instance, abs)
        }
    }
}

#[derive(Clone)]
/// An assumption in the component.
/// Assumed to be true during type checking and validated during code
/// generation.
pub struct Assume {
    cons: Loc<OrderConstraint<Expr>>,
}

impl Assume {
    pub fn new(cons: Loc<OrderConstraint<Expr>>) -> Self {
        Assume { cons }
    }

    /// Resolve expression in the assumption
    pub fn resolve(self, bind: &Binding<Expr>) -> Self {
        Assume {
            cons: self.cons.map(|c| OrderConstraint {
                left: c.left.resolve(bind),
                right: c.right.resolve(bind),
                ..c
            }),
        }
    }

    /// Get the position of the assumption
    pub fn pos(&self) -> GPosIdx {
        self.cons.pos()
    }

    /// Attempts to evaluate the assumption to a boolean value.
    /// Does not work if the assumption has not been resolved.
    pub fn force(self) -> bool {
        let (OrderConstraint { left, right, op }, ..) = self.cons.split();
        let l: u64 = left.try_into().unwrap();
        let r: u64 = right.try_into().unwrap();
        match op {
            super::OrderOp::Gt => l > r,
            super::OrderOp::Gte => l >= r,
            super::OrderOp::Eq => l == r,
        }
    }
}

impl std::fmt::Display for Assume {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "assume {}", self.cons)
    }
}

#[derive(Clone)]
/// A Guard expression
pub enum Guard {
    Or(Box<Guard>, Box<Guard>, GPosIdx),
    Port(Port),
}

impl From<Port> for Guard {
    fn from(v: Port) -> Self {
        Self::Port(v)
    }
}
impl Guard {
    pub fn or(g1: Guard, g2: Guard) -> Self {
        Guard::Or(Box::new(g1), Box::new(g2), GPosIdx::UNKNOWN)
    }
}
impl std::fmt::Display for Guard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Guard::Or(g1, g2, _) => write!(f, "{} | {}", g1, g2),
            Guard::Port(p) => write!(f, "{}", p),
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
    /// Optional guard expression.
    pub guard: Option<Guard>,
}

impl Connect {
    pub fn new(dst: Loc<Port>, src: Loc<Port>, guard: Option<Guard>) -> Self {
        Self { dst, src, guard }
    }
}
impl std::fmt::Display for Connect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(g) = &self.guard {
            write!(f, "{} = {} ? {}", self.dst, g, self.src)
        } else {
            write!(f, "{} = {}", self.dst, self.src)
        }
    }
}

#[derive(Clone)]
/// Representation of a finite state machine
pub struct Fsm {
    /// Name of the FSM
    pub name: Id,
    /// Number of states in the FSM
    pub states: u64,
    /// Signal that triggers the FSM
    pub trigger: Port,
}
impl Fsm {
    pub fn new(name: Id, states: u64, trigger: Port) -> Self {
        Self {
            name,
            states,
            trigger,
        }
    }

    /// Returns the state associated with the FSM port
    pub fn state(port: &Id) -> FilamentResult<u64> {
        let split = port.as_ref().split('_').collect_vec();
        if split.len() == 2 {
            let mb_idx = split[1].parse::<u64>();
            if let Ok(idx) = mb_idx {
                return Ok(idx);
            }
        }
        Err(Error::malformed(format!(
            "PortType cannot be converted into FSM state: {}",
            port
        )))
    }

    /// Return the port associated with the given state
    pub fn port(&self, state: u64) -> Port {
        Port::inv_port(
            Loc::unknown(self.name),
            Loc::unknown(Id::from(format!("_{}", state))),
        )
    }
}

impl Display for Fsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "fsm {}[{}]({})", self.name, self.states, self.trigger)
    }
}
impl std::fmt::Debug for Fsm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Clone)]
/// A generative loop:
/// ```
/// for #i in 0..#W { ... }
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

impl Display for ForLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "for #{} in {}..{} {{", self.idx, self.start, self.end)?;
        for cmd in &self.body {
            writeln!(f, "{};", cmd)?;
        }
        write!(f, "}}")
    }
}

impl From<ForLoop> for Command {
    fn from(v: ForLoop) -> Self {
        Self::ForLoop(v)
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

impl Display for If {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "if {} {{", self.cond)?;
        for cmd in &self.then {
            writeln!(f, "{}", cmd)?;
        }
        writeln!(f, "}} else {{")?;
        for cmd in &self.alt {
            writeln!(f, "{}", cmd)?;
        }
        write!(f, "}}")
    }
}

impl From<If> for Command {
    fn from(v: If) -> Self {
        Self::If(v)
    }
}

#[derive(Clone)]
/// The type of the bundle:
/// ```
/// for<#i> @[G+#i, G+#i+1] #W
/// ```
pub struct BundleType {
    /// The name of the parameter for the bundle type
    pub idx: Loc<Id>,
    /// Length of the bundle. The index parameter ranges over [0, len)
    pub len: Loc<Expr>,
    /// Availability interval for the bundle
    pub liveness: Loc<Range>,
    /// Bitwidth of the bundle
    pub bitwidth: Loc<Expr>,
}

impl BundleType {
    pub fn new(
        idx: Loc<Id>,
        len: Loc<Expr>,
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
            len: self.len.map(|e| e.resolve(binding)),
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

    /// Return the type for a bundle that has been offset by the given expression.
    /// For example:
    /// ```
    /// for<#i> @[G+#i, G+#i+1] #W
    /// ```
    /// offset by `#K` becomes
    /// ```
    /// for<#i> @[G+#i+#K, G+#i+1+#K] #W
    /// ```
    pub fn shrink(self, start: Expr, end: Expr) -> Self {
        // Generate the offset by resolving the index of the bundle type with index+offset
        let len = end - start.clone();
        let idx = self.idx.copy();
        let binding = Binding::new(Some((idx, Expr::abs(idx) + start)));
        let mut resolved = self.resolve_exprs(&binding);
        *resolved.len.inner_mut() = len;
        resolved
    }
}

#[derive(Clone)]
/// Represents a bundle of wires with timing guarantees
/// ```
/// bundle f[10]: for<#i> @[G+#i, G+#i+1] #W;
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

    /// Generate a port definition corresponding to a given index
    pub fn index(&self, idx: Expr) -> PortDef {
        let mut bind = Binding::default();
        bind.insert(*self.typ.idx.inner(), idx);
        let liveness = self.typ.liveness.clone();
        PortDef::port(
            Loc::unknown(Id::from("__FAKE_NAME_SHOULD_NOT_BE_USED")),
            liveness.map(|r| r.resolve_exprs(&bind)),
            self.typ.bitwidth.clone(),
        )
    }

    /// Resolve expressions in the Bundle
    pub fn resolve_exprs(self, binding: &Binding<Expr>) -> Self {
        Self {
            typ: self.typ.resolve_exprs(binding),
            ..self
        }
    }
}

impl Display for Bundle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bundle {}[{}]: ", self.name, self.typ.len)?;
        write!(
            f,
            "for<#{}> {} {};",
            self.typ.idx, self.typ.liveness, self.typ.bitwidth
        )
    }
}
