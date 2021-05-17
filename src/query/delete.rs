use crate::{
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use std::{
    fmt::{self, Write},
    marker::PhantomData,
};

use super::{from_item::*, with::WithQuery, FromItem};

pub struct DeleteQuery<T> {
    pub(crate) with: Option<WithQuery>,
    table: PhantomData<T>,
    wheres: Vec<Box<dyn BooleanExpr>>,
    using: Vec<Box<dyn FromItem>>,
    only: bool,
    returning: Option<Box<dyn ValueExpr>>,
}

impl<T> Default for DeleteQuery<T> {
    fn default() -> Self {
        DeleteQuery {
            with: None,
            table: PhantomData,
            wheres: Vec::new(),
            using: Vec::new(),
            only: false,
            returning: None,
        }
    }
}

impl DeleteQuery<()> {
    pub fn from<T: Table>(self) -> DeleteQuery<T> {
        DeleteQuery {
            with: self.with,
            ..Default::default()
        }
    }
}

impl<T: Table> DeleteQuery<T> {
    pub fn only(mut self) -> Self {
        self.only = true;
        self
    }

    pub fn and_where<E>(mut self, expr: E) -> Self
    where
        E: BooleanExpr + 'static,
    {
        self.wheres.push(Box::new(expr));
        self
    }

    pub fn using<F>(mut self, from: F) -> Self
    where
        F: FromItem + 'static,
    {
        self.using.push(Box::new(from));
        self
    }

    pub fn returning<E>(mut self, expr: E) -> Self
    where
        E: ValueExpr + 'static,
    {
        self.returning = Some(Box::new(expr));
        self
    }
}

impl<T: Table> Collectable for DeleteQuery<T> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        use crate::expr::util::collect_delimited;

        if let Some(ref with) = self.with {
            with._collect(w, t)?;
            w.write_str(" ")?; // space before INSERT
        }

        w.write_str("DELETE FROM ")?;
        if self.only {
            w.write_str("ONLY ")?;
        }
        TableRef::<T>::new()._collect(w, t)?;

        if !self.using.is_empty() {
            w.write_str(" USING ")?;
            collect_delimited(&self.using, false, ", ", w, t)?;
        }

        if !self.wheres.is_empty() {
            w.write_str(" WHERE ")?;
            collect_delimited(&self.wheres, self.wheres.len() > 1, " AND ", w, t)?;
        }

        if let Some(ref returning) = self.returning {
            w.write_str(" RETURNING ")?;
            returning._collect(w, t)?;
        }

        Ok(())
    }
}
