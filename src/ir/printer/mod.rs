//! Implements the [DisplayCtx] trait for the [crate::ir::Context] and [crate::ir::Component] types
//! for most things they contain. In general, the printer should only be used when printing the whole program.
//! Otherwise, the [DisplayCtx::display] method is sufficient.
mod comp;
mod display_ctx;
mod expr;
mod prop;

pub use comp::Printer;
pub use display_ctx::DisplayCtx;
