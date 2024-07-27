use crate::{query::SelectQuery, util::collect_delimited};

use super::*;

pub struct InExpr<E> {
    value: E,
    exprs: Vec<Box<dyn ValueExpr>>,
    not: bool,
}

pub trait InExt: ValueExpr + Sized {
    fn in_values<V>(self, values: impl IntoIterator<Item = V>) -> InExpr<Self>
    where
        V: ValueExpr + 'static,
    {
        InExpr {
            value: self,
            exprs: values.into_iter().map(|v| Box::new(v) as Box<dyn ValueExpr>).collect(),
            not: false,
        }
    }

    fn not_in_values<V>(self, values: impl IntoIterator<Item = V>) -> InExpr<Self>
    where
        V: ValueExpr + 'static,
    {
        InExpr {
            value: self,
            exprs: values.into_iter().map(|v| Box::new(v) as Box<dyn ValueExpr>).collect(),
            not: true,
        }
    }

    fn in_query(self, select: SelectQuery) -> InExpr<Self> {
        InExpr {
            value: self,
            exprs: vec![Box::new(select.as_value())],
            not: false,
        }
    }

    fn not_in_query(self, select: SelectQuery) -> InExpr<Self> {
        InExpr {
            value: self,
            exprs: vec![Box::new(select.as_value())],
            not: true,
        }
    }
}

impl<T> InExt for T where T: ValueExpr {}

impl<E: ValueExpr> BooleanExpr for InExpr<E> {}
impl<E: ValueExpr> ValueExpr for InExpr<E> {}
impl<E: ValueExpr> Expr for InExpr<E> {}
impl<E: ValueExpr> Collectable for InExpr<E> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.value._collect(w, t)?;
        if self.not {
            w.write_str(" NOT")?;
        }

        w.write_str(" IN (")?;

        match self.exprs.len() {
            1 => self.exprs[0].collect(w, t)?, // don't bother wrapping
            _ => collect_delimited(&self.exprs, false, ", ", w, t)?,
        }

        w.write_str(")")
    }
}
