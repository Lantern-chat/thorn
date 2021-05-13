use std::collections::HashMap;
use std::fmt::{self, Write};

use crate::{
    as_::RenameError,
    collect::{Collectable, Collector},
    order::Order,
    *,
};

use super::SelectQuery;

pub trait WithableQuery: Collectable {}

pub struct WithQueryBuilder;

#[derive(Clone, Copy)]
enum Materialized {
    NoPreference,
    Materialized,
    NotMaterialized,
}

pub struct WithQuery {
    queries: HashMap<&'static str, Box<dyn WithableQuery>>,
    mat: Materialized,
}

impl WithQuery {
    pub fn named_query<Q>(mut self, name: &'static str, query: Q) -> Result<Self, RenameError>
    where
        Q: WithableQuery + 'static,
    {
        self.queries
            .insert(RenameError::check_name(name)?, Box::new(query));
        Ok(self)
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
        Ok(())
    }
}

//#[rustfmt::skip]
//impl WithQueryBuilder {
//    pub fn as_<Q>(self, query: Q) -> WithQuery<Q>
//    where
//        Q: WithableQuery,
//    {
//        WithQuery { inner: query, mat: Materialized::NoPreference }
//    }
//}
