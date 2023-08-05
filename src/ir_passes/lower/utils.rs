use crate::ir::{
    self, CompIdx, Component, Context, Ctx, EventIdx, ExprIdx, ParamIdx,
    PortIdx,
};
use calyx_ir as calyx;
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

/// Gets the name of the interface port associated with an event, if it exists.
pub(super) fn interface_name(
    idx: EventIdx,
    comp: &Component,
) -> Option<String> {
    if !comp.get(idx).has_interface {
        return None;
    }

    Some(
        comp.src_info
            .as_ref()
            .map(|src| src.interface_ports.get(&idx).unwrap().to_string())
            .unwrap_or_else(|| format!("ev{}", idx.get())),
    )
}

/// Converts an [ir::ExprIdx] into a [calyx::Width].
/// Expects the [ir::ExprIdx] to either be a singular constant or an abstract variable.
pub(super) fn expr_width(idx: ExprIdx, comp: &Component) -> calyx::Width {
    match comp.get(idx) {
        ir::Expr::Param(p) => calyx::Width::Param {
            value: param_name(*p, comp).into(),
        },
        ir::Expr::Concrete(val) => calyx::Width::Const { value: *val },
        ir::Expr::Bin { .. } | ir::Expr::Fn { .. } => {
            comp.internal_error("Port width must be a parameter or constant.")
        }
    }
}

/// Returns the name of an [ir::Param].
pub(super) fn param_name(idx: ParamIdx, comp: &Component) -> String {
    comp.src_info
        .as_ref()
        .map(|src| src.params.get(&idx).unwrap().to_string())
        .unwrap_or_else(|| format!("pr{}", idx.get()))
}

/// Returns the name of an [ir::Port]
pub(super) fn port_name(
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
            .unwrap_or_else(|| format!("p{}", idx.get())),
        ir::PortOwner::Inv { base, .. } => {
            base.apply(|p, c| port_name(p, ctx, c), ctx)
        }
        ir::PortOwner::Local => format!("p{}", idx.get()),
    }
}

/// Returns the name of an [Component]
pub(super) fn comp_name(idx: CompIdx, ctx: &impl Ctx<Component>) -> String {
    ctx.get(idx)
        .src_info
        .as_ref()
        .map(|src| src.name.to_string())
        .unwrap_or_else(|| format!("comp{}", idx.get()))
}

/// Calculates the max states used for every fsm.
pub fn max_states(idx: CompIdx, ctx: &Context) -> LinkedHashMap<EventIdx, u64> {
    let comp = ctx.get(idx);
    let mut max_states = LinkedHashMap::new();

    comp.ports()
        .iter()
        .map(|(_, port)| {
            let live = &port.live;
            assert!(
                live.len.is_const(comp, 1),
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
