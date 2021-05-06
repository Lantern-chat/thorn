use crate::{
    collect::{Collectable, Collector},
    *,
};

use std::{
    fmt::{self, Write},
    marker::PhantomData,
};

use super::{from_item::*, FromItem};

#[derive(Default)]
pub struct SelectQuery {
    exprs: Vec<Box<dyn Expr>>,
    from: Option<Box<dyn FromItem>>,
}

impl SelectQuery {
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
        self.from = Some(Box::new(item) as Box<dyn FromItem>);
        self
    }

    pub fn from_table<T>(self) -> Self
    where
        T: Table,
    {
        self.from(TableRef::<T>::new())
    }

    pub fn join_left_table<T>(mut self) -> Self
    where
        T: Table,
    {
        self.from = Some(Box::new(Join {
            l: self.from.unwrap(),
            r: TableRef::<T>::new(),
            cond: None,
            kind: JoinType::LeftJoin,
        }) as Box<dyn FromItem>);
        self
    }

    pub fn join_left_table_on<T, E>(mut self, cond: E) -> Self
    where
        T: Table,
        E: Expr + 'static,
    {
        self.from = Some(Box::new(Join {
            l: self.from.unwrap(),
            r: TableRef::<T>::new(),
            cond: Some(Box::new(cond)),
            kind: JoinType::LeftJoin,
        }) as Box<dyn FromItem>);
        self
    }
}

impl Collectable for SelectQuery {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("SELECT ")?;

        let mut exprs = self.exprs.iter();

        if let Some(e) = exprs.next() {
            e._collect(w, t)?;
        }

        for e in exprs {
            w.write_str(", ")?;
            e._collect(w, t)?;
        }

        if let Some(ref from) = self.from {
            w.write_str(" FROM ")?;
            from.collect(w, t)?;
        }

        Ok(())
    }
}
