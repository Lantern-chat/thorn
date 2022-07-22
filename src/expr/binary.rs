use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Power,
    BitAnd,
    BitOr,
    BitXor,
    BitshiftLeft,
    BitshiftRight,
    Concat,
}

#[rustfmt::skip]
pub trait BinaryExt: Expr + Sized {
    #[inline]
    fn add<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Add }
    }
    #[inline]
    fn sub<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Sub }
    }
    #[inline]
    fn mul<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Mul }
    }
    #[inline]
    fn div<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Div }
    }
    #[inline]
    fn rem<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Rem }
    }
    #[inline]
    fn power<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Power }
    }
    #[inline]
    fn bit_and<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::BitAnd }
    }
    #[inline]
    fn bit_or<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::BitOr }
    }
    #[inline]
    fn bit_xor<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::BitXor }
    }
    #[inline]
    fn bitshift_left<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::BitshiftLeft }
    }
    #[inline]
    fn bitshift_right<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::BitshiftRight }
    }
    #[inline]
    fn concat<Rhs>(self, rhs: Rhs) -> BinaryExpr<Self, Rhs> {
        BinaryExpr { lhs: self, rhs, op: BinaryOp::Concat }
    }
}

impl<T> BinaryExt for T where T: ValueExpr {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BinaryExpr<Lhs, Rhs> {
    lhs: Lhs,
    rhs: Rhs,
    op: BinaryOp,
}

impl<Lhs: ValueExpr, Rhs: ValueExpr> ValueExpr for BinaryExpr<Lhs, Rhs> {}
impl<Lhs: ValueExpr, Rhs: ValueExpr> Expr for BinaryExpr<Lhs, Rhs> {}
impl<Lhs: ValueExpr, Rhs: ValueExpr> Collectable for BinaryExpr<Lhs, Rhs> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.lhs._collect(w, t)?;

        w.write_str(" ")?;
        w.write_str(match self.op {
            BinaryOp::Add => "+",
            BinaryOp::Sub => "-",
            BinaryOp::Mul => "*",
            BinaryOp::Div => "/",
            BinaryOp::Rem => "%",
            BinaryOp::Power => "^",
            BinaryOp::BitAnd => "&",
            BinaryOp::BitOr => "|",
            BinaryOp::BitXor => "#",
            BinaryOp::BitshiftLeft => "<<",
            BinaryOp::BitshiftRight => ">>",
            BinaryOp::Concat => "||",
        })?;

        w.write_str(" ")?;
        self.rhs._collect(w, t)
    }
}
