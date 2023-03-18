use super::*;

pub trait BitsExt: Sized + ValueExpr {
    fn has_any_bits<E>(self, bits: E) -> CompExpr<BinaryExpr<Self, E>, Literal>
    where
        BinaryExpr<Self, E>: Expr,
    {
        self.bitand(bits).not_equals(0.lit())
    }

    fn has_all_bits<E>(self, bits: E) -> CompExpr<BinaryExpr<Self, E>, E>
    where
        BinaryExpr<Self, E>: Expr,
        E: Clone,
    {
        self.bitand(bits.clone()).equals(bits)
    }

    fn has_no_bits<E>(self, bits: E) -> CompExpr<BinaryExpr<Self, E>, Literal>
    where
        BinaryExpr<Self, E>: Expr,
    {
        self.bitand(bits).equals(0.lit())
    }

    fn bit_difference<E>(self, bits: E) -> BinaryExpr<Self, UnaryExpr<E>>
    where
        E: ValueExpr,
    {
        self.bitand(bits.bit_not())
    }
}

impl<T> BitsExt for T where T: Sized + ValueExpr {}

pub struct BitsExt2;

impl BitsExt2 {
    pub fn has_all_bits<F1, F2, E1, E2>(
        this: (F1, F2),
        bits: (E1, E2),
    ) -> CompExpr<CompExpr<BinaryExpr<F1, E1>, E1>, CompExpr<BinaryExpr<F2, E2>, E2>>
    where
        F1: ValueExpr,
        F2: ValueExpr,
        E1: ValueExpr + Clone,
        E2: ValueExpr + Clone,
    {
        this.0.has_all_bits(bits.0).and(this.1.has_all_bits(bits.1))
    }
}
