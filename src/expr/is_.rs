use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum IsOp {
    IsNull,
    IsNotNull,
    IsTrue,
    IsNotTrue,
    IsFalse,
    IsNotFalse,
    IsUnknown,
    IsNotUnknown,
}

#[rustfmt::skip]
pub trait IsExt: Expr + Sized {
    #[inline]
    fn is_null(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsNull }
    }
    #[inline]
    fn is_not_null(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsNotNull }
    }
    #[inline]
    fn is_true(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsTrue }
    }
    #[inline]
    fn is_not_true(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsNotTrue }
    }
    #[inline]
    fn is_false(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsFalse }
    }
    #[inline]
    fn is_not_false(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsNotFalse }
    }
    #[inline]
    fn is_unknown(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsUnknown }
    }
    #[inline]
    fn is_not_unknown(self) -> IsExpr<Self> {
        IsExpr { value: self, op: IsOp::IsNotUnknown }
    }
}

impl<T> IsExt for T where T: Expr {}

pub struct IsExpr<V> {
    value: V,
    op: IsOp,
}

impl<V: Expr> Expr for IsExpr<V> {}
impl<V: Expr> Collectable for IsExpr<V> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.value._collect(w, t)?;
        w.write_str(match self.op {
            IsOp::IsNull => " IS NULL",
            IsOp::IsNotNull => " IS NOT NULL",
            IsOp::IsTrue => " IS TRUE",
            IsOp::IsNotTrue => " IS NOT TRUE",
            IsOp::IsFalse => " IS FALSE",
            IsOp::IsNotFalse => " IS NOT FALSE",
            IsOp::IsUnknown => " IS UNKNOWN",
            IsOp::IsNotUnknown => " IS NOT UNKNOWN",
        })
    }
}
