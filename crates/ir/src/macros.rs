#[macro_export]
/// Creates a constructor function for a binary operator.
/// Example: ```
/// construct_binop!(
/// <impl Ctx<Expr>>(ExprIdx::bin, Expr) => Expr;
///     add = ast::Op::Add;
///     mul = ast::Op::Mul;
///     div = ast::Op::Div;
///     rem = ast::Op::Mod;
/// );
/// ```
macro_rules! construct_binop {
    (<$ctx: ty> ($constructor: expr, $in: ty) => $out: ty;
    $($name:ident = $op: expr);* ;) => {
        impl $in {
            $(pub fn $name(self, other: $in, ctx: &mut $ctx) -> $crate::utils::Idx<$out> {
                ctx.add($constructor(
                    self, other, $op
                ))
            })*
        }
    };
}
