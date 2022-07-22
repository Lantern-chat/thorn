use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UnaryOp {
    Not,
    Neg,
    BitNot,
    Abs,
    Sqrt,
    Cbrt,
}

#[rustfmt::skip]
pub trait UnaryExt: Expr + Sized {
    fn not(self) -> UnaryExpr<Self> {
        UnaryExpr { value: self, op: UnaryOp::Not }
    }
    fn neg(self) -> UnaryExpr<Self> {
        UnaryExpr { value: self, op: UnaryOp::Neg }
    }
    fn bit_not(self) -> UnaryExpr<Self> {
        UnaryExpr { value: self, op: UnaryOp::BitNot }
    }
    fn abs(self) -> UnaryExpr<Self> {
        UnaryExpr { value: self, op: UnaryOp::Abs }
    }
    fn square_root(self) -> UnaryExpr<Self> {
        UnaryExpr { value: self, op: UnaryOp::Sqrt }
    }
    fn cube_root(self) -> UnaryExpr<Self> {
        UnaryExpr { value: self, op: UnaryOp::Cbrt }
    }
}

impl<T> UnaryExt for T where T: ValueExpr {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnaryExpr<V> {
    value: V,
    op: UnaryOp,
}

impl<V: ValueExpr> ValueExpr for UnaryExpr<V> {}
impl<V: ValueExpr> Expr for UnaryExpr<V> {}
impl<V: ValueExpr> Collectable for UnaryExpr<V> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str(match self.op {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "0 - ",
            UnaryOp::BitNot => "~",
            UnaryOp::Abs => "@",
            UnaryOp::Sqrt => "|/",
            UnaryOp::Cbrt => "||/",
        })?;

        self.value._collect(w, t)
    }

    fn needs_wrapping(&self) -> bool {
        true
    }
}
