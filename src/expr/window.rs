use crate::util::collect_delimited;

use super::*;

pub struct Window<E, W> {
    func: E,
    window: W,
    filters: Vec<Box<dyn BooleanExpr>>,
}

impl<E, W> Window<E, W> {
    pub fn and_where<C>(mut self, condition: C) -> Self
    where
        C: BooleanExpr + 'static,
    {
        self.filters.push(Box::new(condition));
        self
    }
}

pub trait WindowExpr: Collectable {
    fn prefix(&self) -> &'static str;
}

impl<E: Expr> WindowExpr for OrderExpr<E> {
    fn prefix(&self) -> &'static str {
        "ORDER BY "
    }
}

pub trait WindowExt: Expr + Sized {
    fn over<W: WindowExpr>(self, window: W) -> Window<Self, W> {
        Window {
            func: self,
            window,
            filters: Vec::new(),
        }
    }
}

impl WindowExt for Call {}

impl<E: WindowExt, W: WindowExpr> ValueExpr for Window<E, W> {}
impl<E: WindowExt, W: WindowExpr> Expr for Window<E, W> {}
impl<E: WindowExt, W: WindowExpr> Collectable for Window<E, W> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.func._collect(w, t)?;

        if !self.filters.is_empty() {
            w.write_str(" FILTER WHERE (")?;
            collect_delimited(&self.filters, false, " AND ", w, t)?;
            w.write_str(")")?;
        }

        w.write_str(" OVER (")?;
        w.write_str(self.window.prefix())?;
        self.window._collect(w, t)?;
        w.write_str(")")
    }
}
