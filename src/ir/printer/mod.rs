//! Implements the [DisplayCtx] trait for the [crate::ir::Context] and [crate::ir::Component] types
//! for most things they contain. In general, the printer should only be used when printing the whole program.
//! Otherwise, the [DisplayCtx::display] method is sufficient.
mod display_ctx;
mod expr;
mod printer;
mod prop;

pub use display_ctx::DisplayCtx;
pub use printer::Printer;
