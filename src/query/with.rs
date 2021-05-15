use std::fmt::{self, Write};
use std::{collections::HashMap, marker::PhantomData};

use crate::{
    as_::RenameError,
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use super::SelectQuery;

pub trait WithableQuery: Collectable {}

pub struct WithQueryBuilder;

#[derive(Default)]
pub struct WithQuery {
    queries: HashMap<&'static str, Box<dyn Collectable>>,
    recursive: bool,
}

impl WithQuery {
    pub fn recursive(mut self) -> Self {
        self.recursive = true;
        self
    }

    pub fn with<T: Table, Q>(mut self, query: NamedQuery<T, Q>) -> Self
    where
        Q: WithableQuery + 'static,
    {
        self.queries.insert(T::NAME.name(), Box::new(query));
        self
    }

    pub(crate) fn froms<'a>(&'a self) -> impl Iterator<Item = &'static str> + 'a {
        self.queries.keys().cloned()
    }

    pub fn select(self) -> SelectQuery {
        let mut select = SelectQuery::default();
        select.with = Some(self);
        select
    }
}

impl Collectable for WithQuery {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        let mut queries = self.queries.values();

        if let Some(query) = queries.next() {
            if self.recursive {
                w.write_str("WITH RECURSIVE ")?;
            } else {
                w.write_str("WITH ")?;
            }

            query._collect(w, t)?;

            for query in queries {
                w.write_str(", ")?;
                query._collect(w, t)?;
            }
        }

        Ok(())
    }
}

pub trait TableAsExt: Table {
    fn as_query<Q>(query: Q) -> NamedQuery<Self, Q>
    where
        Q: WithableQuery,
    {
        NamedQuery {
            table: PhantomData,
            query,
        }
    }
}

impl<T> TableAsExt for T where T: Table {}

pub struct NamedQuery<T, Q> {
    table: PhantomData<T>,
    query: Q,
}

impl<T: Table, Q: WithableQuery> Collectable for NamedQuery<T, Q> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str(T::NAME.name())?;
        w.write_str(" AS ")?;

        self.query._collect(w, t)
    }
}

pub struct MaterializedQuery<Q> {
    query: Q,
    materialized: bool,
}

impl<Q> MaterializedQuery<Q> {
    pub fn materialized(mut self) -> Self {
        self.materialized = true;
        self
    }

    pub fn not_materialized(mut self) -> Self {
        self.materialized = false;
        self
    }
}

impl<Q: WithableQuery> WithableQuery for MaterializedQuery<Q> {}
impl<Q: WithableQuery> Collectable for MaterializedQuery<Q> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str(match self.materialized {
            true => "MATERIALIZED ",
            false => "NOT MATERIALIZED ",
        })?;
        self.query._collect(w, t)
    }
}

pub trait WithableQueryExt: WithableQuery + Sized {
    fn materialized(self) -> MaterializedQuery<Self> {
        MaterializedQuery {
            query: self,
            materialized: true,
        }
    }

    fn not_materialized(self) -> MaterializedQuery<Self> {
        MaterializedQuery {
            query: self,
            materialized: false,
        }
    }
}

impl<T> WithableQueryExt for T where T: WithableQuery {}

impl WithableQuery for SelectQuery {}
