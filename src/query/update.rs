use crate::{
    collect::{Collectable, Collector},
    order::Order,
    table::Column,
    *,
};

use std::{
    fmt::{self, Write},
    marker::PhantomData,
};

use super::{
    from_item::*,
    with::{NamedQuery, WithQuery, WithableQuery},
    FromItem,
};

pub(crate) enum Value {
    Default,
    Value(Box<dyn ValueExpr>),
}

pub struct UpdateQuery<T> {
    pub(crate) with: Option<WithQuery>,
    table: PhantomData<T>,
    froms: Vec<Box<dyn FromItem>>,
    only: bool,
    wheres: Vec<Box<dyn BooleanExpr>>,
    sets: Vec<(Box<dyn Column>, Value)>,
    returning: Option<Box<dyn ValueExpr>>,
}

impl<T> Default for UpdateQuery<T> {
    fn default() -> Self {
        UpdateQuery {
            with: None,
            table: PhantomData,
            froms: Vec::new(),
            only: false,
            wheres: Vec::new(),
            sets: Vec::new(),
            returning: None,
        }
    }
}

impl UpdateQuery<()> {
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

    pub fn table<T: Table>(self) -> UpdateQuery<T> {
        UpdateQuery {
            with: self.with,
            only: self.only,
            ..Default::default()
        }
    }
}

impl<T> UpdateQuery<T> {
    pub fn only(mut self) -> Self {
        self.only = true;
        self
    }
}

impl<T: Table> UpdateQuery<T> {
    pub fn from_table<Q: Table>(self) -> Self {
        self.from(TableRef::<Q>::new())
    }

    pub fn from<F>(mut self, from: F) -> Self
    where
        F: FromItem + 'static,
    {
        self.froms.push(Box::new(from));
        self
    }

    pub fn and_where<E>(mut self, cond: E) -> Self
    where
        E: BooleanExpr + 'static,
    {
        self.wheres.push(Box::new(cond));
        self
    }

    pub fn set<C, E>(mut self, column: C, value: E) -> Self
    where
        C: Column + 'static,
        E: ValueExpr + 'static,
    {
        self.sets.push((Box::new(column), Value::Value(Box::new(value))));
        self
    }

    pub fn set_default<C>(mut self, column: C) -> Self
    where
        C: Column + 'static,
    {
        self.sets.push((Box::new(column), Value::Default));
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

impl<T: Table> Collectable for UpdateQuery<T> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        use crate::expr::util::collect_delimited;

        if let Some(ref with) = self.with {
            with._collect(w, t)?;
            w.write_str(" ")?; // space before INSERT
        }

        w.write_str("UPDATE ")?;
        if self.only {
            w.write_str("ONLY ")?;
        }
        TableRef::<T>::new()._collect(w, t)?;

        let mut sets = self.sets.iter();
        if let Some((col, val)) = sets.next() {
            write!(w, " SET \"{}\" = ", col.name())?;
            val.collect(w, t)?;

            for (col, val) in sets {
                write!(w, ", \"{}\" = ", col.name())?;
                val.collect(w, t)?;
            }
        }

        // FROM source [, ...]
        if !self.froms.is_empty() {
            w.write_str(" FROM ")?;
            collect_delimited(&self.froms, false, ", ", w, t)?;
        }

        // WITH named AS ... FROM named
        if let Some(ref with) = self.with {
            with.collect_froms(self.froms.is_empty(), w, t)?;
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

impl Collectable for Value {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        match self {
            Value::Default => w.write_str("DEFAULT"),
            Value::Value(value) => value._collect(w, t),
        }
    }
}
