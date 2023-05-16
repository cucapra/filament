use crate::ast;
pub trait Pass {
    fn transform(ns: ast::Namespace) -> ast::Namespace;
}
