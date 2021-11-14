use std::fmt::{self, Write};
use std::{collections::HashMap, marker::PhantomData};

use crate::{
    as_::RenameError,
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use super::{DeleteQuery, InsertQuery, SelectQuery, UpdateQuery};

pub trait WithableQuery: Collectable {}

pub struct WithQueryBuilder;

#[derive(Default)]
pub struct WithQuery {
    pub(crate) queries: HashMap<&'static str, (bool, Box<dyn Collectable>)>,
    pub(crate) recursive: bool,
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
        self.queries
            .insert(T::NAME.name(), (query.exclude, Box::new(query)));
        self
    }

    pub(crate) fn froms<'a>(&'a self) -> impl Iterator<Item = &'static str> + 'a {
        self.queries
            .iter()
            .filter_map(|(k, v)| (!v.0).then(|| k))
            .cloned()
    }

    pub(crate) fn collect_froms(
        &self,
        from_prefix: bool,
        w: &mut dyn Write,
        _t: &mut Collector,
    ) -> fmt::Result {
        let mut froms = self.froms();
        if let Some(from) = froms.next() {
            if from_prefix {
                w.write_str(" FROM ")?;
            } else {
                w.write_str(", ")?;
            }

            w.write_str("\"")?;
            w.write_str(from)?;
            w.write_str("\"")?;
            for from in froms {
                w.write_str(", \"")?;
                w.write_str(from)?;
                w.write_str("\"")?;
            }
        }

        Ok(())
    }

    pub fn select(self) -> SelectQuery {
        let mut select = SelectQuery::default();
        select.with = Some(self);
        select
    }

    pub fn insert(self) -> InsertQuery<()> {
        let mut insert = InsertQuery::default();
        insert.with = Some(self);
        insert
    }

    pub fn delete(self) -> DeleteQuery<()> {
        let mut delete = DeleteQuery::default();
        delete.with = Some(self);
        delete
    }

    pub fn update(self) -> UpdateQuery<()> {
        let mut update = UpdateQuery::default();
        update.with = Some(self);
        update
    }
}

impl Collectable for WithQuery {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        let mut queries = self.queries.values();

        if let Some((_, query)) = queries.next() {
            if self.recursive {
                w.write_str("WITH RECURSIVE ")?;
            } else {
                w.write_str("WITH ")?;
            }

            query._collect(w, t)?;

            for (_, query) in queries {
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
            exclude: false,
            query,
        }
    }
}

impl<T> TableAsExt for T where T: Table {}

pub struct NamedQuery<T, Q> {
    pub(crate) table: PhantomData<T>,
    pub(crate) exclude: bool,
    pub(crate) query: Q,
}

impl<T, Q> NamedQuery<T, Q> {
    pub fn exclude(mut self) -> Self {
        self.exclude = true;
        self
    }
}

impl<T: Table, Q: WithableQuery> Collectable for NamedQuery<T, Q> {
    fn collect(&self, w: &mut dyn Write, t: &mut Collector) -> fmt::Result {
        w.write_str("\"")?;
        w.write_str(T::NAME.name())?;
        w.write_str("\" AS ")?;

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
impl<T: Table> WithableQuery for InsertQuery<T> {}
impl<T: Table> WithableQuery for UpdateQuery<T> {}
impl<T: Table> WithableQuery for DeleteQuery<T> {}
