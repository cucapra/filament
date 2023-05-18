use super::{
    Ctx, Event, EventIdx, Expr, ExprIdx, Fact, Indexed, InstIdx, Instance,
    InvIdx, Invoke, Param, ParamIdx, Port, PortIdx, Prop, SmallIndexed, Time,
    TimeIdx,
};
use crate::utils::Idx;

#[derive(Default)]
pub struct Component {
    // Component defined values. Once created, we don't expect too many of these
    // to be created.
    /// Ports and bundles defined by the component.
    pub(super) ports: SmallIndexed<Port>,
    /// Parameters defined the component
    pub(super) params: SmallIndexed<Param>,
    /// Events defined by the component
    pub(super) events: SmallIndexed<Event>,

    /// Instances defined by the component
    pub(super) instances: SmallIndexed<Instance>,
    /// Invocations defined by the component
    pub(super) invocations: SmallIndexed<Invoke>,

    /// Facts in the component.
    /// All nested facts are hoisted out to the top context by adding the path
    /// condition as the antecedant.
    pub(super) facts: Vec<Fact>,

    // Interned data. We store this on a per-component basis because events with the
    // same identifiers in different components are not equal.
    /// Interned expressions
    pub(super) exprs: Indexed<Expr>,
    /// Interned times
    pub(super) times: Indexed<Time>,
    /// Interned propositions
    pub(super) props: Indexed<Prop>,
}

impl Component {
    /// Add a number to the context and get handle to it.
    pub fn num(&mut self, n: u64) -> ExprIdx {
        self.exprs.add(Expr::Concrete(n))
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
        self.exprs.add(val)
    }

    fn get(&self, idx: ExprIdx) -> &Expr {
        self.exprs.get(idx)
    }
}

impl Ctx<Time> for Component {
    fn add(&mut self, val: Time) -> TimeIdx {
        self.times.add(val)
    }

    fn get(&self, idx: TimeIdx) -> &Time {
        self.times.get(idx)
    }
}

impl Ctx<Prop> for Component {
    fn add(&mut self, val: Prop) -> Idx<Prop> {
        self.props.add(val)
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
