mod binding;
mod binding_ctx;
pub mod checker;
mod visit;
pub use binding::{Bindings, ResolvedInstance};
pub use binding_ctx::{
    BoundInstance, BoundInvoke, CompBinding, InstBind, InstIdx, InvIdx,
    ProgBinding, SigIdx,
};
pub use visit::Transform;
