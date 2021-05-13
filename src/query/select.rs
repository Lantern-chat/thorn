use crate::{
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use std::fmt::{self, Write};

use super::{from_item::*, with::WithQuery, FromItem};

#[derive(Default)]
pub struct SelectQuery {
    pub(crate) with: Option<WithQuery>,
    on: Vec<Box<dyn Expr>>,
    exprs: Vec<Box<dyn Expr>>,
    froms: Vec<Box<dyn FromItem>>,
    wheres: Vec<Box<dyn Expr>>,
    //groups: Vec<Box<dyn Expr>>, // TODO
    distinct: Option<DistinctMode>,
    having: Vec<Box<dyn Expr>>,
    limit: Option<Box<dyn Expr>>,
    offset: Option<Box<dyn Expr>>,
    orders: Vec<(Order, Box<dyn Expr>)>,
}

impl SelectQuery {
    pub fn distinct(mut self) -> Self {
        self.distinct = Some(DistinctMode::Distinct);
        self
    }

    pub fn on<E>(mut self, expr: E) -> Self
    where
        E: Expr + 'static,
    {
        self.on.push(Box::new(expr));
        self
    }

    pub fn col<C>(self, column: C) -> Self
    where
        C: Table,
    {
        self.expr(ColumnRef(column))
    }

    pub fn cols<C>(self, columns: impl IntoIterator<Item = C>) -> Self
    where
        C: Table,
    {
        self.exprs(columns.into_iter().map(ColumnRef))
    }

    pub fn expr<E>(mut self, expression: E) -> Self
    where
        E: Expr + 'static,
    {
        self.exprs.push(Box::new(expression));
        self
    }

    pub fn exprs<E>(mut self, expressions: impl IntoIterator<Item = E>) -> Self
    where
        E: Expr + 'static,
    {
        self.exprs
            .extend(expressions.into_iter().map(|e| Box::new(e) as Box<dyn Expr>));
        self
    }

    pub fn from<F>(mut self, item: F) -> Self
    where
        F: FromItem + 'static,
    {
        self.froms.push(Box::new(item));
        self
    }

    pub fn from_table<T>(self) -> Self
    where
        T: Table,
    {
        self.from(TableRef::<T>::new())
    }

    pub fn and_where<E>(mut self, cond: E) -> Self
    where
        E: BooleanExpr + 'static,
    {
        self.wheres.push(Box::new(cond));
        self
    }

    pub fn limit<E>(mut self, expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        self.limit = Some(Box::new(expr));
        self
    }

    pub fn limit_n(mut self, limit: i64) -> Self {
        self.limit = Some(Box::new(Literal::Int8(limit)));
        self
    }

    pub fn having<E>(mut self, cond: E) -> Self
    where
        E: BooleanExpr + 'static,
    {
        self.having.push(Box::new(cond));
        self
    }

    pub fn offset<E>(mut self, start: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        self.offset = Some(Box::new(start));
        self
    }

    pub fn offset_n(mut self, start: i64) -> Self {
        self.offset = Some(Box::new(Literal::Int8(start)));
        self
    }

    pub fn order_by<E>(mut self, order: OrderExpr<E>) -> Self
    where
        E: Expr + 'static,
    {
        self.orders.push((order.order, Box::new(order.inner)));
        self
    }
}

impl Collectable for SelectQuery {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        if let Some(ref with) = self.with {
            with._collect(w, t)?;
        }

        w.write_str("SELECT ")?;

        if Some(DistinctMode::Distinct) == self.distinct {
            w.write_str("DISTINCT ")?;

            // DISTINCT ON
            let mut ons = self.on.iter();
            if let Some(expr) = ons.next() {
                w.write_str("ON (")?;
                expr._collect(w, t)?;
            }
            for expr in ons {
                w.write_str(", ")?;
                expr._collect(w, t)?;
            }
            if !self.on.is_empty() {
                w.write_str(")")?;
            }
        }

        // SELECT [expressions [, ...]]
        let mut exprs = self.exprs.iter();
        if let Some(e) = exprs.next() {
            e._collect(w, t)?;
            for e in exprs {
                w.write_str(", ")?;
                e._collect(w, t)?;
            }
        }

        // FROM
        let mut froms = self.froms.iter();
        if let Some(from) = froms.next() {
            w.write_str(" FROM ")?;
            from._collect(w, t)?;
            for from in froms {
                w.write_str(", ")?;
                from._collect(w, t)?;
            }
        }

        // WITH named AS ... FROM named
        if let Some(ref with) = self.with {
            if self.froms.is_empty() {
                w.write_str(" FROM ")?;
            } else {
                w.write_str(", ")?;
            }

            let mut froms = with.froms();

            if let Some(from) = froms.next() {
                w.write_str(from)?;
            }

            for from in froms {
                w.write_str(", ")?;
                w.write_str(from)?;
            }
        }

        // WHERE
        let mut wheres = self.wheres.iter();
        if let Some(cond) = wheres.next() {
            w.write_str(" WHERE ")?;
            let where_wrapped = self.wheres.len() > 1;
            if where_wrapped {
                w.write_str("(")?;
            }
            cond._collect(w, t)?;
            for cond in wheres {
                w.write_str(" AND ")?;
                cond._collect(w, t)?;
            }
            if where_wrapped {
                w.write_str(")")?;
            }
        }

        // HAVING
        let mut conds = self.having.iter();
        if let Some(cond) = conds.next() {
            w.write_str(" HAVING ")?;
            let having_wrapped = self.having.len() > 1;
            if having_wrapped {
                w.write_str("(")?;
            }
            cond._collect(w, t)?;
            for cond in conds {
                w.write_str(" AND ")?;
                cond._collect(w, t)?;
            }
            if having_wrapped {
                w.write_str(")")?;
            }
        }

        let mut orders = self.orders.iter();
        if let Some((order, ref inner)) = orders.next() {
            w.write_str(" ORDER BY ")?;
            OrderExpr { order: *order, inner }._collect(w, t)?; // reconstruct and collect

            for (order, ref inner) in orders {
                w.write_str(", ")?;
                OrderExpr { order: *order, inner }._collect(w, t)?; // reconstruct and collect
            }
        }

        // LIMIT
        if let Some(ref limit) = self.limit {
            w.write_str(" LIMIT ")?;
            limit._collect(w, t)?;
        }

        // OFFSET
        if let Some(ref offset) = self.offset {
            w.write_str(" OFFSET ")?;
            offset._collect(w, t)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistinctMode {
    All,
    Distinct,
}

pub struct SelectValue {
    value: SelectQuery,
}

impl ValueExpr for SelectValue {}
impl Expr for SelectValue {}
impl Collectable for SelectValue {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        self.value._collect(w, t)
    }
}
