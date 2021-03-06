use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BetweenOp {
    Between,
    NotBetween,
    BetweenSymmetric,
    NotBetweenSymmetric,
}

#[rustfmt::skip]
pub trait BetweenExt: Expr + Sized {
    #[inline]
    fn between<A, B>(self, a: A, b: B) -> BetweenExpr<Self, A, B> {
        BetweenExpr { x: self, a, b, op: BetweenOp::Between }
    }

    #[inline]
    fn not_between<A, B>(self, a: A, b: B) -> BetweenExpr<Self, A, B> {
        BetweenExpr { x: self, a, b, op: BetweenOp::NotBetween }
    }

    #[inline]
    fn between_symmetric<A, B>(self, a: A, b: B) -> BetweenExpr<Self, A, B> {
        BetweenExpr { x: self, a, b, op: BetweenOp::BetweenSymmetric }
    }

    #[inline]
    fn not_between_symmetric<A, B>(self, a: A, b: B) -> BetweenExpr<Self, A, B> {
        BetweenExpr { x: self, a, b, op: BetweenOp::NotBetweenSymmetric }
    }
}

impl<T> BetweenExt for T where T: ValueExpr {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BetweenExpr<X, A, B> {
    x: X,
    a: A,
    b: B,
    op: BetweenOp,
}

impl<X: ValueExpr, A: ValueExpr, B: ValueExpr> BooleanExpr for BetweenExpr<X, A, B> {}
impl<X: ValueExpr, A: ValueExpr, B: ValueExpr> ValueExpr for BetweenExpr<X, A, B> {}
impl<X: ValueExpr, A: ValueExpr, B: ValueExpr> Expr for BetweenExpr<X, A, B> {}
impl<X: ValueExpr, A: ValueExpr, B: ValueExpr> Collectable for BetweenExpr<X, A, B> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.x._collect(w, t)?;

        w.write_str(match self.op {
            BetweenOp::Between => " BETWEEN ",
            BetweenOp::NotBetween => " NOT BETWEEN ",
            BetweenOp::BetweenSymmetric => " BETWEEN SYMMETRIC ",
            BetweenOp::NotBetweenSymmetric => " NOT BETWEEN SYMMETRIC ",
        })?;

        self.a._collect(w, t)?;
        w.write_str(" AND ")?;
        self.b._collect(w, t)
    }
}
