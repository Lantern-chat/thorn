use crate::{
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use std::fmt::{self, Write};

use super::{
    from_item::*,
    with::{NamedQuery, WithQuery, WithableQuery},
    FromItem,
};

#[derive(Clone, Copy)]
enum CombinationType {
    Union,
    UnionAll,
    Intersect,
    IntersectAll,
    Except,
    ExceptAll,
}

#[derive(Default)]
pub struct SelectQuery {
    pub(crate) with: Option<WithQuery>,
    on: Vec<Box<dyn Expr>>,
    exprs: Vec<Box<dyn Expr>>,
    froms: Vec<Box<dyn FromItem>>,
    wheres: Vec<Box<dyn Expr>>,
    groups: Vec<Box<dyn Expr>>,
    distinct: Option<DistinctMode>,
    having: Vec<Box<dyn Expr>>,
    limit: Option<Box<dyn Expr>>,
    offset: Option<Box<dyn Expr>>,
    orders: Vec<(Order, Box<dyn Expr>)>,
    combos: Vec<(CombinationType, SelectQuery)>,
}

impl SelectQuery {
    pub fn as_value(self) -> SelectValue {
        SelectValue { value: self }
    }

    /// Shorthand for `Query::with().with(query).select()...`
    pub fn with<T: Table, Q>(mut self, query: NamedQuery<T, Q>) -> Self
    where
        Q: WithableQuery + 'static,
    {
        self.with = Some(match self.with {
            Some(with) => with.with(query),
            None => Query::with().with(query),
        });

        self
    }

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

    pub fn cols<'a, T>(self, columns: impl IntoIterator<Item = &'a T>) -> Self
    where
        T: Table,
    {
        self.exprs(columns.into_iter().cloned().map(ColumnRef))
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
        self.exprs.extend(expressions.into_iter().map(|e| Box::new(e) as Box<dyn Expr>));
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

    pub fn group_by<E>(mut self, value: E) -> Self
    where
        E: Expr + 'static,
    {
        self.groups.push(Box::new(value));
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
        self.limit = Some(Box::new(Lit(limit)));
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
        self.offset = Some(Box::new(Lit(start)));
        self
    }

    pub fn order_by<E>(mut self, order: OrderExpr<E>) -> Self
    where
        E: Expr + 'static,
    {
        self.orders.push((order.order, Box::new(order.inner)));
        self
    }

    pub fn union(mut self, query: SelectQuery) -> Self {
        self.combos.push((CombinationType::Union, query));
        self
    }

    pub fn union_all(mut self, query: SelectQuery) -> Self {
        self.combos.push((CombinationType::UnionAll, query));
        self
    }

    pub fn intersect(mut self, query: SelectQuery) -> Self {
        self.combos.push((CombinationType::Intersect, query));
        self
    }

    pub fn intersect_all(mut self, query: SelectQuery) -> Self {
        self.combos.push((CombinationType::IntersectAll, query));
        self
    }

    pub fn except(mut self, query: SelectQuery) -> Self {
        self.combos.push((CombinationType::Except, query));
        self
    }

    pub fn except_all(mut self, query: SelectQuery) -> Self {
        self.combos.push((CombinationType::ExceptAll, query));
        self
    }
}

impl Collectable for SelectQuery {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        use crate::expr::util::collect_delimited;

        if let Some(ref with) = self.with {
            with._collect(w, t)?;
            w.write_str(" ")?; // space before SELECT
        }

        w.write_str("SELECT ")?;

        if Some(DistinctMode::Distinct) == self.distinct {
            w.write_str("DISTINCT ")?;

            // DISTINCT ON
            if !self.on.is_empty() {
                w.write_str("ON ")?;
                collect_delimited(&self.on, true, ", ", w, t)?;
            }
        }

        // SELECT [expressions [, ...]]
        collect_delimited(&self.exprs, false, ", ", w, t)?;

        // FROM source [, ...]
        if !self.froms.is_empty() {
            w.write_str(" FROM ")?;
            collect_delimited(&self.froms, false, ", ", w, t)?;
        }

        // WITH named AS ... FROM named
        if let Some(ref with) = self.with {
            with.collect_froms(self.froms.is_empty(), w, t)?;
        }

        // WHERE
        if !self.wheres.is_empty() {
            w.write_str(" WHERE ")?;
            collect_delimited(&self.wheres, self.wheres.len() > 1, " AND ", w, t)?;
        }

        if !self.groups.is_empty() {
            w.write_str(" GROUP BY ")?;
            collect_delimited(&self.groups, self.groups.len() > 1, ", ", w, t)?;
        }

        // HAVING
        if !self.having.is_empty() {
            w.write_str(" HAVING ")?;
            collect_delimited(&self.having, self.having.len() > 1, " AND ", w, t)?;
        }

        // [ { UNION | INTERSECT | EXCEPT } [ ALL | DISTINCT ] select ]
        for (kind, query) in &self.combos {
            assert_eq!(
                self.exprs.len(),
                query.exprs.len(),
                "Unions must produce the same number of columns!"
            );

            w.write_str(match kind {
                CombinationType::Union => " UNION ",
                CombinationType::UnionAll => " UNION ALL ",
                CombinationType::Intersect => " INTERSECT ",
                CombinationType::IntersectAll => " INTERSECT ALL ",
                CombinationType::Except => " EXCEPT ",
                CombinationType::ExceptAll => " EXCEPT ALL ",
            })?;

            query._collect(w, t)?; // wraps automatically
        }

        if !self.orders.is_empty() {
            w.write_str(" ORDER BY ")?;

            let iter = self.orders.iter().map(|(order, inner)| OrderExpr { order: *order, inner });

            collect_delimited(iter, false, ", ", w, t)?;
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

impl Arguments for SelectValue {
    fn to_vec(self) -> Vec<Box<dyn Expr>> {
        assert_eq!(
            1,
            self.value.exprs.len(),
            "Using SELECT as an argument is only valid with a single expression"
        );

        vec![Box::new(self)]
    }
}
