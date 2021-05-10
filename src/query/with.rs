use crate::*;

pub trait WithableQuery: Collectable {}

pub struct WithQueryBuilder;

#[derive(Clone, Copy)]
enum Materialized {
    NoPreference,
    Materialized,
    NotMaterialized,
}

pub struct WithQuery<Q> {
    inner: Q,
    mat: Materialized,
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
