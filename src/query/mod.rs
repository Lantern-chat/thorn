pub mod from_item;
pub use from_item::{FromItem, TableJoinExt};

pub mod select;
pub use select::SelectQuery;

pub mod insert;
pub use insert::InsertQuery;

pub mod call;
pub use call::CallQuery;

use crate::{Call, Collectable};

pub mod with;
pub use with::{TableAsExt, WithQuery, WithableQuery, WithableQueryExt};

pub struct Query;
impl Query {
    pub fn select() -> SelectQuery {
        SelectQuery::default()
    }

    pub fn insert() -> InsertQuery<()> {
        InsertQuery::default()
    }

    pub fn call(proc: Call) -> CallQuery {
        CallQuery::new(proc)
    }

    pub fn with() -> WithQuery {
        WithQuery::default()
    }
}

pub trait AnyQuery: Collectable {}

impl AnyQuery for SelectQuery {}
impl AnyQuery for CallQuery {}
