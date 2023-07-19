use crate::ir::{self, Ctx};
use calyx_ir as calyx;

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

impl ir::EventIdx {
    /// Returns the name of this interface port.
    pub fn interface_name(self, comp: &ir::Component) -> Option<String> {
        if !comp.get(self).has_interface {
            return None;
        }

        Some(if let Some(src) = &comp.src_info {
            src.interface_ports.get(&self).unwrap().to_string()
        } else {
            format!("ev{}", self.get())
        })
    }
}

impl ir::ExprIdx {
    /// Compiles an [ir::ExprIdx] into a [u64].
    /// Expects the [ir::ExprIdx] to be a single constant value, and panics if this isn't the case
    pub fn as_u64(self, comp: &ir::Component) -> u64 {
        self.as_concrete(comp).unwrap_or_else(|| {
            comp.internal_error("Expression must be a constant.")
        })
    }

    /// Converts an [ir::ExprIdx] into a [calyx::Width].
    /// Expects the [ir::ExprIdx] to either be a singular constant or an abstract variable.
    pub fn as_width(self, comp: &ir::Component) -> calyx::Width {
        match comp.get(self) {
            ir::Expr::Param(p) => calyx::Width::Param {
                value: p.name(comp).into(),
            },
            ir::Expr::Concrete(val) => calyx::Width::Const { value: *val },
            ir::Expr::Bin { .. } | ir::Expr::Fn { .. } => comp
                .internal_error("Port width must be a parameter or constant."),
        }
    }
}

impl ir::ParamIdx {
    /// Returns the name of this parameter.
    pub fn name(self, comp: &ir::Component) -> String {
        if let Some(src) = &comp.src_info {
            src.params.get(&self).unwrap().to_string()
        } else {
            format!("pr{}", self.get())
        }
    }
}

impl ir::PortIdx {
    /// Returns the name of this port.
    pub fn name(self, ctx: &ir::Context, comp: &ir::Component) -> String {
        let p = comp.get(self);

        match &p.owner {
            ir::PortOwner::Sig { .. } => {
                if let Some(src) = &comp.src_info {
                    src.ports.get(&self).unwrap().to_string()
                } else {
                    format!("p{}", self.get())
                }
            }
            ir::PortOwner::Inv { base, .. } => {
                base.apply(ctx, |c, p| p.name(ctx, c))
            }
            ir::PortOwner::Local => format!("p{}", self.get()),
        }
    }
}

impl ir::CompIdx {
    /// Gets the name of this component
    pub fn name(self, ctx: &impl Ctx<ir::Component>) -> String {
        match &ctx.get(self).src_info {
            Some(src_info) => src_info.name.to_string(),
            None => format!("comp{}", self.get()),
        }
    }
}

impl From<&ir::Direction> for calyx::Direction {
    fn from(value: &ir::Direction) -> Self {
        match value {
            ir::Direction::In => calyx::Direction::Input,
            ir::Direction::Out => calyx::Direction::Output,
        }
    }
}
