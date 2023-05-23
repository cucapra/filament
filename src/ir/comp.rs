use super::{
    Ctx, Event, EventIdx, Expr, ExprIdx, Fact, IndexStore, InstIdx, Instance,
    Interned, InvIdx, Invoke, Param, ParamIdx, Port, PortIdx, Prop, Time,
    TimeIdx,
};
use crate::utils::Idx;

#[derive(Default)]
pub struct Component {
    // Component defined values.
    /// Ports and bundles defined by the component.
    pub(super) ports: IndexStore<Port>,
    /// Parameters defined the component
    pub(super) params: IndexStore<Param>,
    /// Events defined by the component
    pub(super) events: IndexStore<Event>,

    /// Instances defined by the component
    pub(super) instances: IndexStore<Instance>,
    /// Invocations defined by the component
    pub(super) invocations: IndexStore<Invoke>,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    pub(super) exprs: Interned<Expr>,
    /// Interned times
    pub(super) times: Interned<Time>,
    /// Interned propositions
    pub(super) props: Interned<Prop>,
}

impl Component {
    /// Add a number to the context and get handle to it.
    pub fn num(&mut self, n: u64) -> ExprIdx {
        self.exprs.intern(Expr::Concrete(n))
    }
}

impl Ctx<Port> for Component {
    fn add(&mut self, val: Port) -> PortIdx {
        self.ports.add(val)
    }

    fn get(&self, idx: PortIdx) -> &Port {
        self.ports.get(idx)
    }
}

impl Ctx<Param> for Component {
    fn add(&mut self, val: Param) -> ParamIdx {
        self.params.add(val)
    }

    fn get(&self, idx: ParamIdx) -> &Param {
        self.params.get(idx)
    }
}

impl Ctx<Event> for Component {
    fn add(&mut self, val: Event) -> EventIdx {
        self.events.add(val)
    }

    fn get(&self, idx: EventIdx) -> &Event {
        self.events.get(idx)
    }
}

impl Ctx<Instance> for Component {
    fn add(&mut self, val: Instance) -> InstIdx {
        self.instances.add(val)
    }

    fn get(&self, idx: InstIdx) -> &Instance {
        self.instances.get(idx)
    }
}

impl Ctx<Invoke> for Component {
    fn add(&mut self, val: Invoke) -> InvIdx {
        self.invocations.add(val)
    }

    fn get(&self, idx: InvIdx) -> &Invoke {
        self.invocations.get(idx)
    }
}

impl Ctx<Expr> for Component {
    fn add(&mut self, val: Expr) -> ExprIdx {
        self.exprs.intern(val)
    }

    fn get(&self, idx: ExprIdx) -> &Expr {
        self.exprs.get(idx)
    }
}

impl Ctx<Time> for Component {
    fn add(&mut self, val: Time) -> TimeIdx {
        self.times.intern(val)
    }

    fn get(&self, idx: TimeIdx) -> &Time {
        self.times.get(idx)
    }
}

impl Ctx<Prop> for Component {
    fn add(&mut self, val: Prop) -> Idx<Prop> {
        self.props.intern(val)
    }

    fn get(&self, idx: Idx<Prop>) -> &Prop {
        self.props.get(idx)
    }
}

// We can use indexing syntax for all values in the context for which it is a Ctx.
impl<K> std::ops::Index<Idx<K>> for Component
where
    Component: Ctx<K>,
{
    type Output = K;

    fn index(&self, index: Idx<K>) -> &Self::Output {
        self.get(index)
    }
}
