mod compile;
mod fsm_gen;

pub(super) type TimeRep = crate::core::FsmIdxs;
pub(super) use fsm_gen::Fsm;

pub use compile::compile;
