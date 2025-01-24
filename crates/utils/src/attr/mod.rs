mod attributes;
mod ctx;
mod store;
mod types;

pub use attributes::Attributes;
pub use ctx::AttrCtx;
pub use store::AttrStore;
pub use types::{
    comp_attrs::{Attrs as CompAttrs, Bool as CompBool, Num as CompNum},
    port_attrs::{Attrs as PortAttrs, Bool as PortBool, Num as PortNum},
};
