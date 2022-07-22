use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NullOrder {
    None,
    First,
    Last,
}

// TODO: USING operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Order {
    Ascending(NullOrder),
    Descending(NullOrder),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderExpr<E> {
    pub(crate) inner: E,
    pub(crate) order: Order,
}

impl<E> OrderExpr<E> {
    pub fn nulls_first(mut self) -> Self {
        self.order = match self.order {
            Order::Ascending(_) => Order::Ascending(NullOrder::First),
            Order::Descending(_) => Order::Descending(NullOrder::First),
        };
        self
    }
    pub fn nulls_last(mut self) -> Self {
        self.order = match self.order {
            Order::Ascending(_) => Order::Ascending(NullOrder::Last),
            Order::Descending(_) => Order::Descending(NullOrder::Last),
        };
        self
    }
}

#[rustfmt::skip]
pub trait OrderExt: Expr + Sized {
    fn ascending(self) -> OrderExpr<Self> {
        OrderExpr { inner: self, order: Order::Ascending(NullOrder::None) }
    }
    fn descending(self) -> OrderExpr<Self> {
        OrderExpr { inner: self, order: Order::Descending(NullOrder::None) }
    }
}
impl<T> OrderExt for T where T: Expr {}

// NOTE: This does NOT implement Expr itself
impl<E: Expr> Collectable for OrderExpr<E> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.inner._collect(w, t)?;
        w.write_str(match self.order {
            Order::Ascending(NullOrder::None) => " ASC",
            Order::Descending(NullOrder::None) => " DESC",
            Order::Ascending(NullOrder::First) => " ASC NULLS FIRST",
            Order::Descending(NullOrder::First) => " DESC NULLS FIRST",
            Order::Ascending(NullOrder::Last) => " ASC NULLS LAST",
            Order::Descending(NullOrder::Last) => " DESC NULLS LAST",
        })
    }
}
