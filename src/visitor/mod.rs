mod binding_ctx;
mod checker;
mod visit;

pub use binding_ctx::{
    BoundInstance, BoundInvoke, CompBinding, InstIdx, InvIdx, ProgBinding,
    SigIdx,
};
pub use checker::{Checker, Traverse};
pub use visit::Transform;
