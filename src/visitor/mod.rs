mod binding;
mod binding_ctx;
mod checker;
mod visit;
pub use binding::{Bindings, ResolvedInstance};
pub use binding_ctx::{
    BoundInstance, BoundInvoke, CompBinding, InstBind, InstIdx, InvIdx,
    ProgBinding, SigIdx,
};
pub use checker::Checker;
pub use visit::Transform;
