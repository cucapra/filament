use crate::{Expr, Prop, Time};

use super::{
    Access, CompIdx, Component, Ctx, Event, ExprIdx, Fact, Foreign, InfoIdx,
    InstIdx, InvIdx, ParamIdx, PortIdx, PropIdx, Range, TimeIdx, TimeSub,
};

#[derive(Clone, PartialEq, Eq)]
/// A flattened and minimized representation of the control flow graph.
/// Bundle definitions and facts are removed during the process of compilation to the IR.
pub enum Command {
    /// Instance
    Instance(InstIdx),
    /// Invocation
    Invoke(InvIdx),
    /// Definition of a bundle
    BundleDef(PortIdx),
    /// A wire connection
    Connect(Connect),
    /// A `let`-bound parameter
    Let(Let),
    /// A `for` loop
    ForLoop(Loop),
    /// An `if` statement
    If(If),
    /// An `assume` or `assert` fact
    Fact(Fact),
    /// An `exists` binding
    Exists(Exists),
}
impl Command {
    pub fn is_loop(&self) -> bool {
        matches!(self, Command::ForLoop(_loop))
    }

    pub fn is_if(&self) -> bool {
        matches!(self, Command::If(_if))
    }
}

impl From<InstIdx> for Command {
    fn from(idx: InstIdx) -> Self {
        Command::Instance(idx)
    }
}
impl From<InvIdx> for Command {
    fn from(idx: InvIdx) -> Self {
        Command::Invoke(idx)
    }
}
impl From<PortIdx> for Command {
    fn from(idx: PortIdx) -> Self {
        Command::BundleDef(idx)
    }
}
impl From<Connect> for Command {
    fn from(con: Connect) -> Self {
        Command::Connect(con)
    }
}
impl From<Loop> for Command {
    fn from(loop_: Loop) -> Self {
        Command::ForLoop(loop_)
    }
}
impl From<If> for Command {
    fn from(if_: If) -> Self {
        Command::If(if_)
    }
}
impl From<Fact> for Command {
    fn from(fact: Fact) -> Self {
        Command::Fact(fact)
    }
}
impl From<Exists> for Command {
    fn from(exists: Exists) -> Self {
        Command::Exists(exists)
    }
}
impl From<Let> for Command {
    fn from(let_: Let) -> Self {
        Command::Let(let_)
    }
}

#[derive(Clone, PartialEq, Eq)]
/// An instantiated component
pub struct Instance {
    /// The component being instantiated
    pub comp: CompIdx,
    /// The parameters used in the binding of this instance
    pub args: Box<[ExprIdx]>,
    /// The active range of this instance
    pub lives: Vec<Range>,
    /// The parameters defined by this instance
    pub params: Vec<ParamIdx>,
    /// The information associated with this instance
    pub info: InfoIdx,
}

impl InstIdx {
    /// Gets the component being instantiated
    pub fn comp(self, ctx: &impl Ctx<Instance>) -> CompIdx {
        let inst = ctx.get(self);
        inst.comp
    }

    pub fn relevant_vars(
        self,
        ctx: &(impl Ctx<Instance> + Ctx<Expr> + Ctx<Time> + Ctx<Prop>),
    ) -> Vec<ParamIdx> {
        let Instance { args, lives, .. } = ctx.get(self);
        args.iter()
            .flat_map(|arg| arg.relevant_vars(ctx).into_iter())
            .chain(lives.iter().flat_map(|r| {
                r.start
                    .relevant_vars(ctx)
                    .into_iter()
                    .chain(r.end.relevant_vars(ctx))
            }))
            .collect()
    }
}

#[derive(Clone, PartialEq, Eq)]
/// A connection between two ports
pub struct Connect {
    pub src: Access,
    pub dst: Access,
    pub info: InfoIdx,
}

#[derive(Clone, PartialEq, Eq)]
/// An invocation of a component
/// Unlike in the AST, invocations are completely desuarged and do not have any
/// ports.
/// The ports are represented as connections.
pub struct Invoke {
    /// The instance being invoked
    pub inst: InstIdx,
    /// The event bindings defined by the invocation
    pub events: Vec<EventBind>,
    // The ports defined by this invocation
    pub ports: Vec<PortIdx>,
    // The information associated with this invocation
    pub info: InfoIdx,
}

impl InvIdx {
    /// The instance being invoked
    pub fn inst(self, ctx: &impl Ctx<Invoke>) -> InstIdx {
        let inv = ctx.get(self);
        inv.inst
    }

    /// The times the invoke uses, along with the EventBind infos
    pub fn times(
        self,
        ctx: &impl Ctx<Invoke>,
    ) -> impl Iterator<Item = (TimeIdx, InfoIdx)> + '_ {
        let inv = ctx.get(self);
        inv.events.iter().map(|eb| (eb.arg, eb.info))
    }

    /// Get the component being invoked
    pub fn comp<C>(self, ctx: &C) -> CompIdx
    where
        C: Ctx<Instance> + Ctx<Invoke>,
    {
        let inst = self.inst(ctx);
        inst.comp(ctx)
    }
}

#[derive(Clone, PartialEq, Eq)]
/// A loop over a range of numbers
pub struct Loop {
    pub index: ParamIdx,
    pub start: ExprIdx,
    pub end: ExprIdx,
    pub body: Vec<Command>,
}

#[derive(Clone, PartialEq, Eq)]
/// A conditional statement
pub struct If {
    pub cond: PropIdx,
    pub then: Vec<Command>,
    pub alt: Vec<Command>,
}

#[derive(Clone, PartialEq, Eq)]
/// Binding for an event argument of an invocation
pub struct EventBind {
    /// The delay of the event being provided for the binding
    pub delay: TimeSub,
    /// The binding for the event
    pub arg: TimeIdx,
    /// Information for the event
    pub info: InfoIdx,
    /// The event for which we provide the binding.
    pub base: Foreign<Event, Component>,
}

impl EventBind {
    pub fn new(
        delay: TimeSub,
        arg: TimeIdx,
        info: InfoIdx,
        base: Foreign<Event, Component>,
    ) -> Self {
        Self {
            delay,
            arg,
            info,
            base,
        }
    }
}

/// Binding for an existentially quantified parameter
#[derive(Clone, PartialEq, Eq)]
pub struct Exists {
    /// The existentially quantified parameter
    pub param: ParamIdx,
    /// The binding for the parameter
    pub expr: ExprIdx,
}

/// A `let`-bound parameter
#[derive(Clone, PartialEq, Eq)]
pub struct Let {
    /// The parameter
    pub param: ParamIdx,
    /// The binding for the parameter
    pub expr: Option<ExprIdx>,
}
