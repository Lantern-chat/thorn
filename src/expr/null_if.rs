use super::*;

pub struct NullIfExpr<A, B> {
    a: A,
    b: B,
}

pub trait NullIfExt: Expr + Sized {
    fn null_if<B>(self, b: B) -> NullIfExpr<Self, B> {
        NullIfExpr { a: self, b }
    }
}
impl<T> NullIfExt for T where T: Expr {}

impl<A: Expr, B: Expr> Expr for NullIfExpr<A, B> {}
impl<A: Expr, B: Expr> Collectable for NullIfExpr<A, B> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("NULLIF(")?;
        self.a._collect(w, t)?;
        w.write_str(", ")?;
        self.b._collect(w, t)?;
        w.write_str(")")
    }
}
