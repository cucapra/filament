use super::{Event, Expr, Param, Port, Range, Time};
use crate::idx;

pub type ExprIdx = idx!(Expr);
pub type ParamIdx = idx!(Param);
pub type PortIdx = idx!(Port);
pub type EventIdx = idx!(Event);
pub type TimeIdx = idx!(Time);
pub type RangeIdx = idx!(Range);
