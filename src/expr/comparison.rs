use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ComparisonOp {
    LessThan,
    GreaterThan,
    LessThanEqual,
    GreaterThanEqual,
    Equal,
    NotEqual,
}

pub trait ComparableExpr: Collectable {}
impl<T> ComparableExpr for T where T: Expr {}

impl<T> CompExt for T where T: Expr {}
#[rustfmt::skip]
pub trait CompExt: Expr + Sized {
    #[inline]
    fn less_than<Rhs>(self, rhs: Rhs) -> CompExpr<Self, Rhs> {
        CompExpr { lhs: self, rhs, op: ComparisonOp::LessThan }
    }
    #[inline]
    fn greater_than<Rhs>(self, rhs: Rhs) -> CompExpr<Self, Rhs> {
        CompExpr { lhs: self, rhs, op: ComparisonOp::GreaterThan }
    }
    #[inline]
    fn less_than_equal<Rhs>(self, rhs: Rhs) -> CompExpr<Self, Rhs> {
        CompExpr { lhs: self, rhs, op: ComparisonOp::LessThanEqual }
    }
    #[inline]
    fn greater_than_equal<Rhs>(self, rhs: Rhs) -> CompExpr<Self, Rhs> {
        CompExpr { lhs: self, rhs, op: ComparisonOp::GreaterThanEqual }
    }
    #[inline]
    fn equals<Rhs>(self, rhs: Rhs) -> CompExpr<Self, Rhs> {
        CompExpr { lhs: self, rhs, op: ComparisonOp::Equal }
    }
    #[inline]
    fn not_equals<Rhs>(self, rhs: Rhs) -> CompExpr<Self, Rhs> {
        CompExpr { lhs: self, rhs, op: ComparisonOp::NotEqual }
    }
}

pub struct CompExpr<Lhs, Rhs> {
    lhs: Lhs,
    rhs: Rhs,
    op: ComparisonOp,
}

impl<Lhs: ComparableExpr, Rhs: ComparableExpr> Expr for CompExpr<Lhs, Rhs> {}
impl<Lhs: ComparableExpr, Rhs: ComparableExpr> Collectable for CompExpr<Lhs, Rhs> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.lhs._collect(w, t)?;

        w.write_str(" ")?;
        w.write_str(match self.op {
            ComparisonOp::LessThan => "<",
            ComparisonOp::GreaterThan => ">",
            ComparisonOp::LessThanEqual => "<=",
            ComparisonOp::GreaterThanEqual => ">=",
            ComparisonOp::Equal => "=",
            ComparisonOp::NotEqual => "<>",
        })?;

        w.write_str(" ")?;
        self.rhs._collect(w, t)
    }
}
