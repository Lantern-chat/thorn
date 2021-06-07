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

enum Values {
    Values(Vec<Box<dyn ValueExpr>>),
    Query(Box<dyn ValueExpr>),
}

pub struct InsertQuery<T> {
    pub(crate) with: Option<WithQuery>,
    cols: Vec<T>,
    values: Values,
    returning: Vec<Box<dyn Expr>>,
}

impl<T> Default for InsertQuery<T> {
    fn default() -> Self {
        InsertQuery {
            with: Default::default(),
            cols: Default::default(),
            values: Values::Values(Vec::new()),
            returning: Default::default(),
        }
    }
}

impl InsertQuery<()> {
    pub fn with<W: Table, Q>(mut self, query: NamedQuery<W, Q>) -> Self
    where
        Q: WithableQuery + 'static,
    {
        self.with = Some(match self.with {
            Some(with) => with.with(query),
            None => Query::with().with(query),
        });

        self
    }

    pub fn into<T: Table>(self) -> InsertQuery<T> {
        InsertQuery {
            with: self.with,
            ..InsertQuery::<T>::default()
        }
    }
}

impl<T: Table> InsertQuery<T> {
    pub fn cols<'a>(mut self, cols: impl IntoIterator<Item = &'a T>) -> Self {
        self.cols.extend(cols.into_iter().cloned());
        self
    }

    pub fn values<E>(mut self, exprs: impl IntoIterator<Item = E>) -> Self
    where
        E: ValueExpr + 'static,
    {
        match self.values {
            Values::Values(ref mut values) => {
                values.extend(exprs.into_iter().map(|e| Box::new(e) as Box<dyn ValueExpr>));
            }
            Values::Query(_) => panic!("Cannot insert both values and query results!"),
        }

        self
    }

    pub fn value<E>(mut self, expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        match self.values {
            Values::Values(ref mut values) => values.push(Box::new(expr)),
            Values::Query(_) => panic!("Cannot insert both values and query results!"),
        }

        self
    }

    pub fn query<E>(mut self, expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        self.values = match self.values {
            Values::Values(ref values) if values.is_empty() => Values::Query(Box::new(expr)),
            Values::Query(_) => panic!("Cannot insert more than one query!"),
            Values::Values(_) => panic!("Cannot insert both values and query results!"),
        };
        self
    }

    pub fn returning<E>(mut self, expr: E) -> Self
    where
        E: Expr + 'static,
    {
        self.returning.push(Box::new(expr));
        self
    }
}

impl<T: Table> Collectable for InsertQuery<T> {
    fn needs_wrapping(&self) -> bool {
        true
    }

    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        use crate::expr::util::collect_delimited;

        if let Some(ref with) = self.with {
            with._collect(w, t)?;
            w.write_str(" ")?; // space before INSERT
        }

        w.write_str("INSERT INTO ")?;

        TableRef::<T>::new()._collect(w, t)?;

        // print column names without table prefix
        let mut cols = self.cols.iter();
        if let Some(col) = cols.next() {
            w.write_str(" (\"")?;
            w.write_str(col.name())?;

            for col in cols {
                w.write_str("\", \"")?;
                w.write_str(col.name())?;
            }
            w.write_str("\")")?;
        }

        match self.values {
            Values::Values(ref values) => {
                if values.is_empty() {
                    w.write_str(" DEFAULT VALUES")?;
                } else {
                    if !self.cols.is_empty() {
                        assert_eq!(
                            values.len(),
                            self.cols.len(),
                            "Columns and Values must be equal length!"
                        );
                    }

                    w.write_str(" VALUES ")?;
                    collect_delimited(values, true, ", ", w, t)?;
                }
            }
            Values::Query(ref query) => query._collect(w, t)?,
        }

        if !self.returning.is_empty() {
            w.write_str(" RETURNING ")?;
            collect_delimited(&self.returning, false, ", ", w, t)?;
        }

        Ok(())
    }
}
