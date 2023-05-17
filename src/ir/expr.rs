use super::{ExprIdx, ParamIdx};
use crate::ast;

#[derive(PartialEq, Eq, Hash, Clone, Debug)]
pub enum Expr {
    Param(ParamIdx),
    Concrete(u64),
    Bin {
        op: ast::Op,
        lhs: ExprIdx,
        rhs: ExprIdx,
    },
    Fn {
        op: ast::UnFn,
        args: Box<[ExprIdx]>,
    },
}

impl std::ops::Add for ExprIdx {
    type Output = Expr;

    fn add(self, rhs: Self) -> Self::Output {
        Expr::Bin {
            op: ast::Op::Add,
            lhs: self,
            rhs,
        }
    }
}
