use super::*;

pub trait BitsExt: Sized + ValueExpr {
    fn has_any_bits<E>(self, bits: E) -> CompExpr<BinaryExpr<Self, E>, Literal>
    where
        BinaryExpr<Self, E>: Expr,
    {
        self.bit_and(bits).not_equals(0.lit())
    }

    fn has_all_bits<E>(self, bits: E) -> CompExpr<BinaryExpr<Self, E>, E>
    where
        BinaryExpr<Self, E>: Expr,
        E: Clone,
    {
        self.bit_and(bits.clone()).equals(bits)
    }

    fn has_no_bits<E>(self, bits: E) -> CompExpr<BinaryExpr<Self, E>, Literal>
    where
        BinaryExpr<Self, E>: Expr,
    {
        self.bit_and(bits).equals(0.lit())
    }

    fn bit_difference<E>(self, bits: E) -> BinaryExpr<Self, UnaryExpr<E>>
    where
        E: ValueExpr,
    {
        self.bit_and(bits.bit_not())
    }
}

impl<T> BitsExt for T where T: Sized + ValueExpr {}
