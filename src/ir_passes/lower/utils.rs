use crate::ir::{
    self, CompIdx, Component, Context, Ctx, EventIdx, ExprIdx, Info, InfoIdx,
    InstIdx, Instance, ParamIdx, PortIdx,
};
use calyx_ir::{self as calyx, RRC};
use linked_hash_map::LinkedHashMap;

type AttrPair = (calyx::Attribute, u64);
/// A set of interface ports that are required for all components.
pub(super) const INTERFACE_PORTS: [(AttrPair, (&str, u64, calyx::Direction));
    2] = [
    (
        (calyx::Attribute::Bool(calyx::BoolAttr::Clk), 1),
        ("clk", 1, calyx::Direction::Input),
    ),
    (
        (calyx::Attribute::Bool(calyx::BoolAttr::Reset), 1),
        ("reset", 1, calyx::Direction::Input),
    ),
];

/// Helper struct that generates names for [crate::utils::Idx]s given their [Ctx].
pub(super) struct NameGenerator {
    use_info: bool,
}

impl NameGenerator {
    pub fn new(use_info: bool) -> Self {
        Self { use_info }
    }

    /// Helper function to generate the name of an [crate::utils::Idx] using its info if the debug flag is enabled.
    fn info_name(&self, idx: InfoIdx, ctx: &impl Ctx<Info>) -> Option<String> {
        self.use_info.then(|| {
            format!("{}_{}", idx.get_name(ctx).unwrap_or_default(), idx.get())
        })
    }

    /// Gets the name of the interface port associated with an event, if it exists.
    pub fn interface_name(
        &self,
        idx: EventIdx,
        comp: &Component,
    ) -> Option<String> {
        let ev = comp.get(idx);

        ev.has_interface.then(|| {
            comp.src_info
                .as_ref()
                .map(|src| src.interface_ports.get(&idx).unwrap().to_string())
                .or_else(|| self.info_name(ev.info, comp))
                .unwrap_or_else(|| format!("ev{}", idx.get()))
        })
    }

    /// Converts an [ir::ExprIdx] into a [calyx::Width].
    /// Expects the [ir::ExprIdx] to either be a singular constant or an abstract variable.
    pub fn expr_width(&self, idx: ExprIdx, comp: &Component) -> calyx::Width {
        match comp.get(idx) {
            ir::Expr::Param(p) => calyx::Width::Param {
                value: self.param_name(*p, comp).into(),
            },
            ir::Expr::Concrete(val) => calyx::Width::Const { value: *val },
            ir::Expr::Bin { .. } | ir::Expr::Fn { .. } => comp
                .internal_error("Port width must be a parameter or constant."),
        }
    }

    /// Returns the name of an [ir::Param].
    pub fn param_name(&self, idx: ParamIdx, comp: &Component) -> String {
        comp.src_info
            .as_ref()
            .map(|src| src.params.get(&idx).unwrap().to_string())
            .or_else(|| self.info_name(comp.get(idx).info, comp))
            .unwrap_or_else(|| format!("pr{}", idx.get()))
    }

    /// Returns the name of an [ir::Port]
    pub fn port_name(
        &self,
        idx: PortIdx,
        ctx: &Context,
        comp: &Component,
    ) -> String {
        let p = comp.get(idx);

        match &p.owner {
            ir::PortOwner::Sig { .. } => comp
                .src_info
                .as_ref()
                .map(|src| src.ports.get(&idx).unwrap().to_string())
                .or_else(|| self.info_name(comp.get(idx).info, comp))
                .unwrap_or_else(|| format!("p{}", idx.get())),
            ir::PortOwner::Inv { base, .. } => {
                base.apply(|p, c| self.port_name(p, ctx, c), ctx)
            }
            ir::PortOwner::Local => self
                .info_name(comp.get(idx).info, comp)
                .unwrap_or_else(|| format!("p{}", idx.get())),
        }
    }

    /// Returns the name of an [Instance]
    pub fn instance_name<C: Ctx<Instance> + Ctx<Info>>(
        &self,
        idx: InstIdx,
        ctx: &C,
    ) -> String {
        self.info_name(ctx.get(idx).info, ctx)
            .unwrap_or_else(|| format!("inst{}", idx.get()))
    }

    /// Returns the name of a [Component]
    pub fn comp_name(&self, idx: CompIdx, ctx: &impl Ctx<Component>) -> String {
        ctx.get(idx)
            .src_info
            .as_ref()
            .map(|src| src.name.to_string())
            .unwrap_or_else(|| format!("comp{}", idx.get()))
    }
}

/// Calculates the max states used for every fsm for the given component.
pub fn max_states(comp: &Component) -> LinkedHashMap<EventIdx, u64> {
    let mut max_states = LinkedHashMap::new();

    comp.ports()
        .iter()
        .map(|(idx, port)| {
            let live = &port.live;
            assert!(
                idx.is_not_bundle(comp),
                "Bundles should have been compiled away."
            );

            // need only the end here as ends follow starts and all ranges should be represented by a simple offset.
            live.range.end
        })
        .for_each(|idx| {
            let time = comp.get(idx);
            let nv = time.offset.concrete(comp);
            if nv > *max_states.get(&time.event).unwrap_or(&0) {
                max_states.insert(time.event, nv);
            }
        });

    max_states
}

/// Converts a cell to a list of port definitions
pub fn cell_to_port_def(cr: &RRC<calyx::Cell>) -> Vec<calyx::PortDef<u64>> {
    let cell = cr.borrow();
    cell.ports()
        .iter()
        .map(|pr| {
            let port = pr.borrow();
            // Reverse port direction because signature refers to internal interface.
            calyx::PortDef {
                name: port.name,
                width: port.width,
                direction: port.direction.reverse(),
                attributes: port.attributes.clone(),
            }
        })
        .collect()
}
